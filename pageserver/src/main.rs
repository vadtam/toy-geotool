#[macro_use] extern crate rocket;

use std::path::{Path, PathBuf};
use rand::{
    thread_rng, Rng,
    distributions::Alphanumeric,
};
use std::net::SocketAddr;
use std::{thread, time};
use std::time::{SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use regex::Regex;

use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use rocket::http::{Status, ContentType};
//use rocket::response::status;
use rocket::response::stream::TextStream;
use rocket::outcome::{try_outcome};
use rocket::serde::{Serialize, Deserialize, json::Json, json};
use rocket::{
    request::{Outcome, Request, FromRequest},
    form::{Form, Strict, Errors},
    response::Redirect,
    http::{Cookie, CookieJar},
    outcome::IntoOutcome,
    fs::NamedFile,
};
use rocket::form::{self, FromForm};
use rocket_db_pools::{
    sqlx, deadpool_redis,
    Database, Pool, Connection,
    sqlx::{Row, Acquire, Transaction, PgConnection, Postgres},
};
use rocket_dyn_templates::{Template, context};
use deadpool_redis::redis;
use hhmmss::Hhmmss;

mod well;
use well::*;
mod user;
use user::*;
mod company;
use company::*;
mod database;
use database::*;
mod tera;
use tera::*;
mod tag;
use tag::*;
mod point;
use point::*;
mod lttb;
use lttb::*;
mod cycle;
use cycle::*;
mod fourier;
use fourier::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct TextUnits<'a> {
    pressure: &'a str,
    temperature: &'a str,
    rate: &'a str,
    density: &'a str,
    volume: &'a str,
    injectivity: &'a str,
}

fn get_text_units(units: &PresentationUnits) -> TextUnits {
    if *units == PresentationUnits::US {
        TextUnits{ pressure: "psi", temperature: "degC", rate: "bpm",
            density: "ppg", volume: "bbl", injectivity: "bpd/psi"}
    } else if *units == PresentationUnits::EU {
        TextUnits{ pressure: "bar", temperature: "degC", rate: "m3/h",
            density: "sg", volume: "m3", injectivity: "(m3/h)/bar"}
    } else {
        panic!("get_text_units: unknown presentation units!")
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct TextTitles<'a> {
    bhp: &'a str,
    bht: &'a str,
    whp: &'a str,
    rate: &'a str,
    density: &'a str,
    volume: &'a str,
    injectivity: &'a str,
}

fn get_text_titles(units: &PresentationUnits) -> TextTitles {
    if *units == PresentationUnits::US {
        TextTitles{bhp: "Bottom-hole pressure, psi",
                   bht: "Bottom-hole temperature, C",
                   whp: "Well-head pressure, psi",
                   rate: "Injection rate, bpm",
                   density: "Fluid density, ppg",
                   volume: "Injected volume, bbl",
                   injectivity: "Injectivity, bpd/psi"}
    } else if *units == PresentationUnits::EU {
        TextTitles{bhp: "Bottom-hole pressure, bar",
                   bht: "Bottom-hole temperature, C",
                   whp: "Well-head pressure, bar",
                   rate: "Injection rate, m3/h",
                   density: "Fluid density, sg",
                   volume: "Injected volume, m3",
                   injectivity: "Injectivity, (m3/h)/bar"}
    } else {
        panic!("get_text_titles: unknown presentation units!")
    }
}

#[derive(FromForm)]
struct NewTagForm {
    #[field(validate = len(1..60))]
    name: String,
    #[field(name = "value-size")]
    value_size : PointValueSize,
    #[field(name = "units-text")]
    #[field(validate = len(1..20))]
    units_text: String,
    #[field(validate = len(0..1600))]
    description: String,
}

#[derive(FromForm)]
struct EditTagForm {
    #[field(validate = len(1..60))]
    name: String,
    #[field(name = "units-text")]
    #[field(validate = len(1..20))]
    units_text: String,
    #[field(validate = len(0..1600))]
    description: String,
}

#[derive(FromForm)]
struct LoginForm {
    #[field(validate = len(1..20))]
    username: String,
    #[field(validate = len(16..=32))]
    password: String,
    #[field(name = "nextUrl")]
    #[field(validate = len(..200))]
    _next_url: String,
}

fn validate_form_url_field<'v>(url: &str) -> form::Result<'v, ()> {
    lazy_static! {
        static ref RE: Regex = Regex::new("[a-z0-9]{2,32}").unwrap();
    }
    if !RE.is_match(url) {
        Err(form::Error::validation("invalid url field"))?
    } else {
        Ok(())
    }
}

#[derive(FromForm)]
struct NewCompanyForm {
    #[field(validate = validate_form_url_field())]
    #[field(validate = len(1..20))]
    id: String,
    #[field(validate = len(1..60))]
    name: String,
}

#[derive(FromForm)]
struct EditCompanyForm {
    #[field(validate = len(1..60))]
    name: String,
}

fn validate_finite_f64<'v>(value: &f64) -> form::Result<'v, ()> {
    if value.is_finite() && !value.is_subnormal() {
        Ok(())
    } else {
        Err(form::Error::validation("value must be finite"))?
    }
}

fn validate_positive_finite_f64<'v>(value: &f64) -> form::Result<'v, ()> {
    if value.is_finite() && !value.is_subnormal() && (*value > 0.0) {
        Ok(())
    } else {
        Err(form::Error::validation("value must be finite and positive"))?
    }
}

fn validate_nonnegative_finite_f32<'v>(value: &f32) -> form::Result<'v, ()> {
    if value.is_finite() && !value.is_subnormal() && (*value >= 0.0) {
        Ok(())
    } else {
        Err(form::Error::validation("value must be finite and nonnegative"))?
    }
}

fn validate_positive_finite_f32<'v>(value: &f32) -> form::Result<'v, ()> {
    if value.is_finite() && !value.is_subnormal() && (*value > 0.0) {
        Ok(())
    } else {
        Err(form::Error::validation("value must be finite and positive"))?
    }
}

fn validate_positive_option<'v>(option: &Option<f32>) -> form::Result<'v, ()> {
    if option.is_some() {
        let value: f32 = option.unwrap();
        if value.is_finite() && !value.is_subnormal() && (value > 0.0) {
            Ok(())
        } else {
            return Err(form::Error::validation("Optional value must be positive or None."))?
        }
    } else {
        Ok(())
    }
}

#[derive(FromForm)]
struct NewWellForm {
    #[field(validate = validate_form_url_field())]
    #[field(validate = len(1..20))]
    id: String,
    #[field(validate = len(1..60))]
    name: String,
    #[field(validate = validate_positive_option())]
    #[field(name = "initial-reservoir-pressure")]
    initial_reservoir_pressure: Option<f32>,
    #[field(validate = validate_positive_option())]
    #[field(name = "pressure-sensors-height")]
    pressure_sensors_height: Option<f32>,
    units: PresentationUnits,
    #[field(validate = validate_computation_modes(&self.bht_mode,
            &self.whp_mode, &self.rate_mode, &self.rho_mode, &self.vtot_mode,
            &self.ii_mode, &self.pressure_sensors_height))]
    #[field(name = "bhp-mode")]
    bhp_mode: ComputationMode,
    #[field(name = "bht-mode")]
    bht_mode: ComputationMode,
    #[field(name = "whp-mode")]
    whp_mode: ComputationMode,
    #[field(name = "rate-mode")]
    rate_mode: ComputationMode,
    #[field(name = "rho-mode")]
    rho_mode: ComputationMode,
    #[field(name = "vtot-mode")]
    vtot_mode: ComputationMode,
    #[field(name = "ii-mode")]
    ii_mode: ComputationMode,
}

#[derive(FromForm)]
struct EditWellForm {
    #[field(validate = len(1..60))]
    name: String,
    #[field(validate = validate_positive_option())]
    #[field(name = "initial-reservoir-pressure")]
    initial_reservoir_pressure: Option<f32>,
    #[field(validate = validate_positive_option())]
    #[field(name = "pressure-sensors-height")]
    pressure_sensors_height: Option<f32>,
    units: PresentationUnits,
    #[field(validate = validate_computation_modes(&self.bht_mode,
            &self.whp_mode, &self.rate_mode, &self.rho_mode, &self.vtot_mode,
            &self.ii_mode, &self.pressure_sensors_height))]
    #[field(name = "bhp-mode")]
    bhp_mode: ComputationMode,
    #[field(name = "bht-mode")]
    bht_mode: ComputationMode,
    #[field(name = "whp-mode")]
    whp_mode: ComputationMode,
    #[field(name = "rate-mode")]
    rate_mode: ComputationMode,
    #[field(name = "rho-mode")]
    rho_mode: ComputationMode,
    #[field(name = "vtot-mode")]
    vtot_mode: ComputationMode,
    #[field(name = "ii-mode")]
    ii_mode: ComputationMode,
}

#[derive(FromForm)]
struct NewUserForm<'a> {
    #[field(validate = validate_form_url_field())]
    #[field(validate = len(1..20))]
    id: String,
    #[field(name = "first-name")]
    #[field(validate = len(1..60))]
    first_name: &'a str,
    #[field(name = "last-name")]
    #[field(validate = len(1..60))]
    last_name: &'a str,
    #[field(validate = len(1..60))]
    email: &'a str,
    #[field(validate = len(16..200))]
    password1: &'a str,
    #[field(validate = len(16..200))]
    #[field(validate = eq(self.password1))]
    password2: &'a str,
    #[field(name = "web-access")]
    web_access: WebAccess,
    #[field(name = "api-access")]
    api_access: ApiAccess,
    category: UserCategory,
}

#[derive(FromForm)]
struct EditUserForm<'a> {
    #[field(name = "first-name")]
    #[field(validate = len(1..60))]
    first_name: &'a str,
    #[field(name = "last-name")]
    #[field(validate = len(1..60))]
    last_name: &'a str,
    #[field(validate = len(1..60))]
    email: &'a str,
    #[field(name = "web-access")]
    web_access: WebAccess,
    #[field(name = "api-access")]
    api_access: ApiAccess,
    category: UserCategory,
}

#[derive(FromForm)]
struct EditPasswordForm<'a> {
    #[field(validate = len(16..200))]
    password1: &'a str,
    #[field(validate = len(16..200))]
    #[field(validate = eq(self.password1))]
    password2: &'a str,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r User {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        async fn memdb_get_token(
                conn: &mut deadpool_redis::Connection,
                userid: &str) -> Option<String> {
            redis::cmd("GET").arg(&[userid]).query_async(conn).await.unwrap()
        }

        async fn db_get_user(db: &DB, userid: &str) -> Option<User> {
            let mut conn = db.get().await.unwrap();
            let q = "SELECT id,uuid,category,company,first_name,".to_owned() +
                "last_name,email,last_active,web_access,api_access " +
                "FROM public.users WHERE id = $1";
            sqlx::query_as::<_, User>(&q).bind(userid)
                .fetch_optional(&mut conn).await.unwrap()
        }

        async fn db_update_last_active(db: &DB, userid: &str) {
            let mut conn = db.get().await.unwrap();
            let duration_res = SystemTime::now().duration_since(UNIX_EPOCH);
            if duration_res.is_ok() {
                let last_active = duration_res.unwrap().as_secs() as i32;  // from u64
                let q = "UPDATE public.users SET last_active = $2 WHERE id = $1";
                let res = sqlx::query(q).bind(userid)
                    .bind(last_active).execute(&mut *conn).await;
                if res.is_err() {
                    panic!("db_update_last_active: failed");
                }
            }
        }

        let user_result = req.local_cache_async(async {
            let memdb = req.guard::<&MEMDB>().await.succeeded()?;
            let mut redis_conn = memdb.get().await.unwrap();
            let remote_addr: SocketAddr = req.remote()?;

            // compare this ip address with redis records
            let remote_addr: &str = &remote_addr.ip().to_string();
            let strikes: Option<u64> = redis::cmd("GET").
                arg(&[remote_addr]).query_async(&mut *redis_conn).await.unwrap();
            let mut strikes: u64 = match strikes {
                Some(n) => n,
                None => 0
            };
            if strikes >= 10 {
                strikes += 1;
                let strikes_ss = (strikes).to_string();
                redis::cmd("SET").arg(&[&remote_addr, &strikes_ss.as_str(), "EX", "86400"])
                    .query_async::<_, ()>(&mut *redis_conn).await.unwrap();
                let wait_time = time::Duration::from_secs(strikes*3);
                thread::sleep(wait_time);
                let no_user: Option<User> = None;
                return no_user;
            }

            let db = req.guard::<&DB>().await.succeeded()?;
            let userid: String = req.cookies().get_private("userid")?.value().to_string();
            let token_user: String = req.cookies().get_private("token")?.value().to_string();
            let token_db_maybe = memdb_get_token(&mut redis_conn, &userid).await;
            if token_db_maybe.is_none() {
                // increment counter, wait some time before return
                strikes += 1;
                let strikes_ss = (strikes).to_string();
                redis::cmd("SET").arg(&[&remote_addr, &strikes_ss.as_str(), "EX", "86400"])
                    .query_async::<_, ()>(&mut *redis_conn).await.unwrap();
                let wait_time = time::Duration::from_secs(strikes*3);
                thread::sleep(wait_time);
                let no_user: Option<User> = None;
                return no_user;
            }
            let token_db = token_db_maybe.unwrap();
            if token_user == token_db {
                if strikes > 0 {
                    strikes = 0;
                    let strikes_ss = (strikes).to_string();
                    redis::cmd("SET").arg(&[&remote_addr, &strikes_ss.as_str(), "EX", "86400"])
                        .query_async::<_, ()>(&mut *redis_conn).await.unwrap();

                }
                redis::cmd("SET").arg(&[&userid, token_db.as_str(), "EX", "1800"])
                    .query_async::<_, ()>(&mut redis_conn).await.unwrap();
                db_update_last_active(db, &userid).await;
                db_get_user(db, &userid).await
            } else {
                // increment counter, wait some time before return
                strikes += 1;
                let strikes_ss = (strikes).to_string();
                redis::cmd("SET").arg(&[&remote_addr, &strikes_ss.as_str(), "EX", "86400"])
                    .query_async::<_, ()>(&mut *redis_conn).await.unwrap();
                let wait_time = time::Duration::from_secs(strikes*3);
                thread::sleep(wait_time); 
                let no_user: Option<User> = None;
                no_user
            }
        }).await;

        user_result.as_ref().or_forward(())
    }
}

struct OnlyAdmin;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r OnlyAdmin {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let only_admin_result = req.local_cache_async( async {
            let user = try_outcome!(req.guard::<&User>().await);
            if user.category == UserCategory::Admin {
                let n = &OnlyAdmin{};
                Outcome::Success(n)
            } else {
                Outcome::Forward(())
            }
        }).await;

        *only_admin_result
    }
}

struct StaffAdmin;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r StaffAdmin {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let staff_admin_result = req.local_cache_async( async {
            let user = try_outcome!(req.guard::<&User>().await);
            if user.category > UserCategory::User {
                let n = &StaffAdmin{};
                Outcome::Success(n)
            } else {
                Outcome::Forward(())
            }
        }).await;

        *staff_admin_result
    }
}

struct CanWebRead;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r CanWebRead {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let can_web_read_result = req.local_cache_async( async {
            let user = try_outcome!(req.guard::<&User>().await);
            if user.web_access >= WebAccess::Readonly {
                let n = &CanWebRead{};
                Outcome::Success(n)
            } else {
                Outcome::Forward(())
            }
        }).await;

        *can_web_read_result
    }
}

struct CanWebFull;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r CanWebFull {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let can_web_full_result = req.local_cache_async( async {
            let user = try_outcome!(req.guard::<&User>().await);
            if user.web_access == WebAccess::Full {
                let n = &CanWebFull{};
                Outcome::Success(n)
            } else {
                Outcome::Forward(())
            }
        }).await;

        *can_web_full_result
    }
}

struct CanApiRead;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r CanApiRead {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let can_api_read_result = req.local_cache_async( async {
            let user = try_outcome!(req.guard::<&User>().await);
            if user.api_access >= ApiAccess::Readonly {
                let n = &CanApiRead{};
                Outcome::Success(n)
            } else {
                Outcome::Forward(())
            }
        }).await;

        *can_api_read_result
    }
}

struct CanApiFull;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r CanApiFull {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let can_api_full_result = req.local_cache_async( async {
            let user = try_outcome!(req.guard::<&User>().await);
            if user.api_access == ApiAccess::Full {
                let n = &CanApiFull{};
                Outcome::Success(n)
            } else {
                Outcome::Forward(())
            }
        }).await;

        *can_api_full_result
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Company {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        async fn db_get_company(db: &DB, companyid: &str) -> Option<Company> {
            let mut conn = db.get().await.unwrap();
            let q = "SELECT * FROM public.companies WHERE id = $1";
            sqlx::query_as::<_, Company>(q).bind(companyid)
                .fetch_optional(&mut conn).await.unwrap()
        }

        let user = try_outcome!(req.guard::<&User>().await);

        let company_result: &Option<Company> = req.local_cache_async(async {
            let db = req.guard::<&DB>().await.succeeded()?;
            let maybe_param: Option<Result<&str, _>> = req.param(1);
            if maybe_param.is_none() {
                let no_company: Option<Company> = None;
                no_company
            } else {
                let result = maybe_param.unwrap();
                match result {
                    Ok(companyid) => {
                        let maybe_company = db_get_company(db, &companyid).await;
                        match maybe_company {
                            Some(ref company) => {
                                if user.category == UserCategory::User {
                                    if company.id == user.company {
                                        maybe_company  // exists
                                    } else {
                                        let no_company: Option<Company> = None;
                                        no_company
                                    }
                                } else {
                                    maybe_company  // exists
                                }
                            },
                            None => maybe_company // empty
                        }
                    },
                    Err(_e) => {
                        let no_company: Option<Company> = None;
                        no_company
                    }
                }
            }
        }).await;

        company_result.as_ref().or_forward(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Well {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        async fn db_get_well(db: &DB, wellid: &str) -> Option<Well> {
            let mut conn = db.get().await.unwrap();
            let q = "SELECT * FROM public.wells WHERE id = $1";
            sqlx::query_as::<_, Well>(q).bind(wellid)
                .fetch_optional(&mut conn).await.unwrap()
        }

        let user = try_outcome!(req.guard::<&User>().await);
        let company = try_outcome!(req.guard::<&Company>().await);

        let well_result: &Option<Well> = req.local_cache_async(async {
            let db = req.guard::<&DB>().await.succeeded()?;
            let maybe_param: Option<Result<&str, _>> = req.param(3);
            if maybe_param.is_none() {
                let no_well: Option<Well> = None;
                no_well
            } else {
                let result = maybe_param.unwrap();
                match result {
                    Ok(wellid) => {
                        let maybe_well = db_get_well(db, &wellid).await;
                        match maybe_well {
                            Some(ref well) => {
                                if user.category == UserCategory::User {
                                    if(user.company == company.id) &&
                                            (company.id == well.company) {
                                        maybe_well  // exists
                                    } else {
                                        let no_well: Option<Well> = None;
                                        no_well
                                    }
                                } else {
                                    maybe_well  // exists
                                }
                            },
                            None => maybe_well // empty
                        }
                    },
                    Err(_e) => {
                        let no_well: Option<Well> = None;
                        no_well
                    }
                }
            }
        }).await;

        well_result.as_ref().or_forward(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Cycle {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        async fn db_get_cycle(db: &DB, well: &Well, cycleid: i16) -> Option<Cycle> {
            let mut conn = db.get().await.unwrap();
            let q = "SELECT * FROM public.cycles WHERE well = $1 AND id = $2";
            let cycle_maybe = sqlx::query_as::<_, Cycle>(q).bind(well.uuid).bind(cycleid)
                .fetch_optional(&mut conn).await.unwrap();
            if well.units == PresentationUnits::US {
                cycle_maybe
            } else {
                if cycle_maybe.is_some() {
                    let cycle = cycle_maybe.unwrap();
                    Some(convert_cycle(cycle, &PresentationUnits::US, &well.units))
                } else {
                    cycle_maybe
                }
            }
        }

        let user = try_outcome!(req.guard::<&User>().await);
        let company = try_outcome!(req.guard::<&Company>().await);
        let well = try_outcome!(req.guard::<&Well>().await);

        let cycle_result: &Option<Cycle> = req.local_cache_async(async {
            let db = req.guard::<&DB>().await.succeeded()?;
            let maybe_param: Option<Result<&str, _>> = req.param(5);
            if maybe_param.is_none() {
                let no_cycle: Option<Cycle> = None;
                no_cycle
            } else {
                let result = maybe_param.unwrap();
                match result {
                    Ok(cycleid_ss) => {
                        let cycleid_res = cycleid_ss.parse::<i16>();
                        if cycleid_res.is_err() {
                            let no_cycle: Option<Cycle> = None;
                            no_cycle
                        } else {
                            let cycleid: i16 = cycleid_res.unwrap();
                            let maybe_cycle = db_get_cycle(db, &well, cycleid).await;
                            match maybe_cycle {
                                Some(ref cycle) => {
                                    if user.category == UserCategory::User {
                                        if(user.company == company.id) &&
                                                (company.id == well.company) &&
                                                (well.uuid == cycle.well) {
                                            maybe_cycle  // exists
                                        } else {
                                            let no_cycle: Option<Cycle> = None;
                                            no_cycle
                                        }
                                    } else {
                                        maybe_cycle  // exists
                                    }
                                },
                                None => maybe_cycle // empty
                            }
                        }
                    },
                    Err(_e) => {
                        let no_cycle: Option<Cycle> = None;
                        no_cycle
                    }
                }
            }
        }).await;

        cycle_result.as_ref().or_forward(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r CustomTag {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        async fn db_get_tag(db: &DB, well: &Well, tagid: u32) -> Option<CustomTag> {
            let mut conn = db.get().await.unwrap();
            let q = "SELECT * FROM public.custom_tags WHERE well = $1 and id = $2";
            sqlx::query_as::<_, CustomTag>(q).bind(well.uuid).bind(tagid)
                .fetch_optional(&mut conn).await.unwrap()
        }

        let user = try_outcome!(req.guard::<&User>().await);
        let company = try_outcome!(req.guard::<&Company>().await);
        let well = try_outcome!(req.guard::<&Well>().await);

        let tag_result: &Option<CustomTag> = req.local_cache_async(async {
            let db = req.guard::<&DB>().await.succeeded()?;
            let maybe_param: Option<Result<&str, _>> = req.param(5);
            if maybe_param.is_none() {
                let no_tag: Option<CustomTag> = None;
                no_tag
            } else {
                let result = maybe_param.unwrap();
                match result {
                    Ok(tagid_ss) => {
                        let tagid_res = tagid_ss.parse::<u32>();
                        if tagid_res.is_err() {
                            let no_tag: Option<CustomTag> = None;
                            no_tag
                        } else {
                          let tagid: u32 = tagid_res.unwrap();
                          let maybe_tag = db_get_tag(db, &well, tagid).await;
                          match maybe_tag {
                            Some(ref tag) => {
                                if user.category == UserCategory::User {
                                    if(user.company == company.id) &&
                                        (company.id == well.company) &&
                                            (tag.well == well.uuid) {
                                        maybe_tag  // exists
                                    } else {
                                        let no_tag: Option<CustomTag> = None;
                                        no_tag
                                    }
                                } else {
                                    maybe_tag  // exists
                                }
                            },
                            None => maybe_tag  // empty
                          }
                        }
                    },
                    Err(_e) => {
                        let no_tag: Option<CustomTag> = None;
                        no_tag
                    }
                }
            }
        }).await;

        tag_result.as_ref().or_forward(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r TagF32 {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        async fn db_get_tag(db: &DB, well: &Well, tagid: i16) -> Option<CustomTag> {
            let mut conn = db.get().await.unwrap();
            let q = "SELECT * FROM public.custom_tags WHERE well = $1 and id = $2";
            sqlx::query_as::<_, CustomTag>(q).bind(well.uuid).bind(tagid)
                .fetch_optional(&mut conn).await.unwrap()
        }

        let well = try_outcome!(req.guard::<&Well>().await);

        let tag_result: &Option<TagF32> = req.local_cache_async(async {
            let db = req.guard::<&DB>().await.succeeded()?;
            let maybe_param: Option<Result<&str, _>> = req.param(5);
            if maybe_param.is_none() {
                let no_tag: Option<TagF32> = None;
                no_tag
            } else {
                let result = maybe_param.unwrap();
                match result {
                    Ok(tagid_ss) => {
                        let tagid_res = tagid_ss.parse::<i16>();
                        if tagid_res.is_err() {
                            let no_tag: Option<TagF32> = None;
                            no_tag
                        } else {
                          let tagid: i16 = tagid_res.unwrap();
                          if(tagid >= -7) && (tagid <= 7) && (tagid != 0) {
                              if tagid.abs() == 6 {
                                  // f64
                                  let no_tag: Option<TagF32> = None;
                                  no_tag
                              } else {
                                  let tag_f32 = TagF32{id: tagid};
                                  Some(tag_f32)
                              }
                          } else if tagid == 0 {
                              let no_tag: Option<TagF32> = None;
                              no_tag
                          } else {
                              let maybe_custom_tag = db_get_tag(db, &well, tagid.abs()).await;
                              match maybe_custom_tag {
                                  Some(custom_tag) => {
                                      if custom_tag.value_size == PointValueSize::F32 {
                                          let tag_f32 = TagF32{id: tagid};
                                          Some(tag_f32)
                                      } else {
                                          let no_tag: Option<TagF32> = None;
                                          no_tag
                                      }
                                  },
                                  None => {
                                      let no_tag: Option<TagF32> = None;
                                      no_tag
                                  }
                              }
                          }
                        }
                    },
                    Err(_e) => {
                        let no_tag: Option<TagF32> = None;
                        no_tag
                    }
                }
            }
        }).await;

        tag_result.as_ref().or_forward(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r TagF64 {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        async fn db_get_tag(db: &DB, well: &Well, tagid: i16) -> Option<CustomTag> {
            let mut conn = db.get().await.unwrap();
            let q = "SELECT * FROM public.custom_tags WHERE well = $1 and id = $2";
            sqlx::query_as::<_, CustomTag>(q).bind(well.uuid).bind(tagid)
                .fetch_optional(&mut conn).await.unwrap()
        }

        let well = try_outcome!(req.guard::<&Well>().await);

        let tag_result: &Option<TagF64> = req.local_cache_async(async {
            let db = req.guard::<&DB>().await.succeeded()?;
            let maybe_param: Option<Result<&str, _>> = req.param(5);
            if maybe_param.is_none() {
                let no_tag: Option<TagF64> = None;
                no_tag
            } else {
                let result = maybe_param.unwrap();
                match result {
                    Ok(tagid_ss) => { 
                        let tagid_res = tagid_ss.parse::<i16>();
                        if tagid_res.is_err() {
                            let no_tag: Option<TagF64> = None;
                            no_tag
                        } else {
                          let tagid: i16 = tagid_res.unwrap();
                          if(tagid >= -7) && (tagid <= 7) && (tagid != 0) {
                              if tagid.abs() == 6 {
                                  let tag_f64 = TagF64{id: tagid};
                                  Some(tag_f64)
                              } else {
                                  // f32
                                  let no_tag: Option<TagF64> = None;
                                  no_tag
                              }
                          } else if tagid == 0 {
                              let no_tag: Option<TagF64> = None;
                              no_tag
                          } else {
                              let maybe_custom_tag = db_get_tag(db, &well, tagid.abs()).await;
                              match maybe_custom_tag {
                                  Some(custom_tag) => {
                                      if custom_tag.value_size == PointValueSize::F64 {
                                          let tag_f64 = TagF64{id: tagid};
                                          Some(tag_f64)
                                      } else {
                                          let no_tag: Option<TagF64> = None;
                                          no_tag
                                      }
                                  },
                                  None => {
                                      let no_tag: Option<TagF64> = None;
                                      no_tag
                                  }
                              }
                          }
                        }
                    },
                    Err(_e) => {
                        let no_tag: Option<TagF64> = None;
                        no_tag
                    }
                }
            }
        }).await;

        tag_result.as_ref().or_forward(())
    }
}

#[get("/login", format = "text/html")]
fn login_page() -> Template {
    let context = context!{login_page: true, next_url: ""};
    Template::render("top/login", &context)  
}

#[get("/login/<next_url..>", format = "text/html")]
fn login_page_next_url(next_url: PathBuf) -> Template {
    let context = context!{login_page: true, next_url: &next_url};
    Template::render("top/login", &context)  
}

#[get("/companies", format = "text/html")]
async fn companies_page(user: &User, mut conn: Connection<DB>,
                        _cwr: &CanWebRead) -> Template {
    let companies = get_companies(&mut conn, user).await;
    let context = context!{user: user, companies: &companies, back_url: "/companies"};
    Template::render("top/companies", &context)
}

#[get("/companies", format = "text/html", rank = 2)]
fn companies_page_redirect() -> Redirect {
    Redirect::to(uri!(login_page))
}

#[get("/companies/new", format = "text/html")]
async fn new_company_page(user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull) -> Template {
    let context = context!{user: user, is_new_mode: true, back_url: "/companies"};
    Template::render("top/company-form", &context)
}

#[get("/companies/new", format = "text/html", rank = 2)]
async fn new_company_page_redirect() -> Redirect {
    Redirect::to("/login/companies/new".to_owned())
}

#[post("/companies/new", format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_new_company(_user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
        mut conn: Connection<DB>,
        form: Form<Strict<NewCompanyForm>>) -> (Status, (ContentType, &'static str)) {
    let q = "INSERT INTO public.companies VALUES ($1, $2) ON CONFLICT DO NOTHING";
    let n: u64 = sqlx::query(q).bind(&form.id).bind(&form.name)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, ""))
    } else {
        (Status::Conflict, (ContentType::Plain, "Error: ID or name duplication. Rename."))
    }
}

#[get("/companies/<_companyid>/edit", format = "text/html")]
async fn edit_company_page(_companyid: &str, user: &User, _cwr: &CanWebRead,
                           company: &Company) -> Template {
    let context = context!{user: user, is_new_mode: false,
                           company: company, back_url: "/companies"};
    Template::render("top/company-form", &context)
}

#[get("/companies/<companyid>/edit", format = "text/html", rank = 2)]
async fn edit_company_page_redirect(companyid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid + "/edit")
}

#[post("/companies/<_companyid>/edit", format = "application/x-www-form-urlencoded",
    data = "<form>")]
async fn perform_edit_company(_companyid: &str, _user: &User,
        _oa: &OnlyAdmin, _cwf: &CanWebFull,
        mut conn: Connection<DB>, company: &Company,
        form: Form<Result<Strict<EditCompanyForm>, Errors<'_>>>) ->
            (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<EditCompanyForm> = form.as_ref().unwrap();
    let mut tx: Transaction<'_, Postgres> = conn.begin().await.unwrap();
    let q = "UPDATE public.companies SET name = $2 WHERE id = $1";
    let q1res = sqlx::query(q).bind(&company.id).bind(&form.name)
        .execute(&mut *tx).await;
    if q1res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "Error: company name duplication. Use another name.".to_string()))
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "Server error on edit_company query, part 1b.".to_string()))
        }

    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => (Status::Accepted, (ContentType::Plain, "".to_string())),
        Err(_) => (Status::Conflict, (ContentType::Plain,
            "Server error on edit_company query: commit failed.".to_string())),
    }
}

#[delete("/companies/<_companyid>", format = "text/plain")]
async fn perform_delete_company(_companyid: &str, _user: &User,
        _oa: &OnlyAdmin, _cwf: &CanWebFull, mut conn: Connection<DB>,
        company: &Company) -> (Status, (ContentType, &'static str)) {
    if company.id == "geomec" {
        return (Status::Conflict,
            (ContentType::Plain, "Error. Geomec company can not be deleted."))
    }
    let q = "DELETE FROM public.companies WHERE id = $1";
    let n: u64 = sqlx::query(q).bind(&company.id)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, ""))
    } else {
        (Status::Conflict, (ContentType::Plain, "Error. Company does not exist."))
    }
}

#[get("/companies/<_companyid>/wells", format = "text/html")]
async fn wells_page(_companyid: &str, user: &User, _cwr: &CanWebRead,
                    mut conn: Connection<DB>,
                    company: &Company) -> Template {
    let wells = get_wells(&mut conn, company).await;
    let context = context!{user: user, company: company, wells: &wells,
        back_url: "/companies"};
    Template::render("top/wells", &context)
}

#[get("/companies/<companyid>/wells", format = "text/html", rank = 2)]
fn wells_page_redirect(companyid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid + "/wells")
}

#[get("/companies/<companyid>/wells/new", format = "text/html")]
async fn new_well_page(companyid: &str, user: &User, _oa: &OnlyAdmin,
                       _cwf: &CanWebFull,
                       company: &Company) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells";
    let context = context!{user: user, company: company, is_new_mode: true,
        back_url: &back_url};
    Template::render("top/well-form", &context)
}

#[get("/companies/<companyid>/wells/new", format = "text/html", rank = 2)]
async fn new_well_page_redirect(companyid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid + "/wells/new")
}

#[post("/companies/<_companyid>/wells/new", format = "application/x-www-form-urlencoded",
    data = "<form>")]
async fn perform_add_new_well(_companyid: &str, _user: &User, _oa: &OnlyAdmin,
                              _cwf: &CanWebFull,
                              mut conn: Connection<DB>, company: &Company,
        form: Form<Result<Strict<NewWellForm>, Errors<'_>>>) -> (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<NewWellForm> = form.as_ref().unwrap();
    let q = "INSERT INTO public.wells (id,name,company,".to_owned() +
            "initial_reservoir_pressure,pressure_sensors_height," +
            "units,bhp_mode,bht_mode,whp_mode,rate_mode,rho_mode," +
            "vtot_mode,ii_mode,computer_needed,computed_to) " +
            "VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12," +
            "$13,$14,$15) ON CONFLICT DO NOTHING";
    let n: u64 = sqlx::query(&q).bind(&form.id).bind(&form.name).bind(&company.id)
        .bind(&form.initial_reservoir_pressure).bind(&form.pressure_sensors_height)
        .bind(&form.units).bind(&form.bhp_mode).bind(&form.bht_mode)
        .bind(&form.whp_mode).bind(&form.rate_mode).bind(&form.rho_mode)
        .bind(&form.vtot_mode).bind(&form.ii_mode).bind(false).bind(0)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, "".to_string()))
    } else {
        (Status::Conflict, (ContentType::Plain,
                            "Error: ID or name duplication. Rename.".to_string()))
    }
}

#[get("/companies/<companyid>/wells/<_wellid>/edit", format = "text/html")]
async fn edit_well_page(companyid: &str, _wellid: &str,
                        user: &User, _cwr: &CanWebRead, company: &Company,
                        well: &Well) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells";
    let context = context!{user: user, company: company, well: well, is_new_mode: false,
        back_url: &back_url};
    Template::render("top/well-form", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/edit", format = "text/html", rank = 2)]
async fn edit_well_page_redirect(companyid: &str, wellid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid + "/wells/" + wellid + "/edit")
}

#[post("/companies/<_companyid>/wells/<_wellid>/edit",
       format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_edit_well(_companyid: &str, _wellid: &str,
                            _user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
                            mut conn: Connection<DB>,
                            _company: &Company, well: &Well,
        form: Form<Result<Strict<EditWellForm>, Errors<'_>>>) -> (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<EditWellForm> = form.as_ref().unwrap();
    let mut tx: Transaction<'_, Postgres> = conn.begin().await.unwrap();
    let q1 = "UPDATE public.wells SET initial_reservoir_pressure = $2, ".to_owned() +
            "pressure_sensors_height = $3, units = $4, bhp_mode = $5, bht_mode = $6, " +
            "whp_mode = $7, rate_mode = $8, rho_mode = $9, vtot_mode = $10, " +
            "ii_mode = $11 WHERE id = $1";
    let q1res = sqlx::query(&q1).bind(&well.id)
            .bind(&form.initial_reservoir_pressure).bind(&form.pressure_sensors_height)
            .bind(&form.units).bind(&form.bhp_mode).bind(&form.bht_mode)
            .bind(&form.whp_mode).bind(&form.rate_mode).bind(&form.rho_mode)
            .bind(&form.vtot_mode).bind(&form.ii_mode).execute(&mut *tx).await;
    if q1res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "Server error on edit_well query, part 1a.".to_string()))
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "Server error on edit_well query, part 1b.".to_string()))
        }
    }
    if well.name != form.name {
        let q2 = "UPDATE public.wells SET name = $2 WHERE id = $1";
        let q2res = sqlx::query(&q2).bind(&well.id).bind(&form.name).execute(&mut *tx).await;
        if q2res.is_err() {
            let rollback_result = tx.rollback().await;
            if rollback_result.is_ok() {
                return (Status::Conflict, (ContentType::Plain,
                    "Error: well name dublication.".to_string()))
            } else {
                return (Status::Conflict, (ContentType::Plain,
                    "Server error on edit_well query, part 2b.".to_string()))
            }
        }
    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => (Status::Accepted, (ContentType::Plain, "".to_string())),
        Err(_) => (Status::Conflict, (ContentType::Plain,
            "Server error on edit_well query: commit failed.".to_string())),
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>", format = "text/plain")]
async fn perform_delete_well(_companyid: &str, _wellid: &str,
        _user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
         mut conn: Connection<DB>, _company: &Company,
        well: &Well) -> (Status, (ContentType, &'static str)) {
    let q = "DELETE FROM public.wells WHERE id = $1";
    let n: u64 = sqlx::query(q).bind(&well.id)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, ""))
    } else {
        (Status::Conflict, (ContentType::Plain, "Error. Well does not exist."))
    }
}

#[get("/companies/<companyid>/wells/<_wellid>/tags", format = "text/html")]
async fn tags_page(companyid: &str, _wellid: &str,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well) -> Template {
    let tags = get_custom_tags(&mut conn, well).await;
    let text_units = get_text_units(&well.units);
    let back_url = "/companies/".to_owned() + companyid + "/wells";
    let context = context!{user: user, company: company, well: well,
                           tags: tags, text_units: text_units,
                           back_url: &back_url};
    Template::render("top/tags", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/tags", format = "text/html", rank = 2)]
fn tags_page_redirect(companyid: &str, wellid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/tags")
}

#[get("/companies/<companyid>/wells/<wellid>/tags/new", format = "text/html")]
async fn new_tag_page(companyid: &str, wellid: &str,
                      user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
                      company: &Company, well: &Well) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/tags";
    let context = context!{user: user, company: company, well: well, is_new_mode: true,
                           back_url: &back_url};
    Template:: render("top/tag-form", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/tags/new", format = "text/html", rank = 2)]
async fn new_tag_page_redirect(companyid: &str, wellid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/tags/new")
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/new",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_new_tag(_companyid: &str, _wellid: &str,
        _user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
        mut conn: Connection<DB>, _company: &Company, well: &Well,
        form: Form<Result<Strict<NewTagForm>, Errors<'_>>>) -> (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<NewTagForm> = form.as_ref().unwrap();
    let q = "INSERT INTO public.custom_tags VALUES ".to_owned() +
        "($1,(SELECT next_custom_tagid($1)),$2,$3,$4,$5) ON CONFLICT DO NOTHING";
    let n: u64 = sqlx::query(&q).bind(&well.uuid).bind(&form.value_size)
        .bind(&form.units_text).bind(&form.name).bind(&form.description)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, "".to_string()))
    } else {
        (Status::Conflict, (ContentType::Plain,
            "Error: tag name must be unique.".to_string()))
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>", format = "text/plain")]
async fn perform_delete_tag(_companyid: &str, _wellid: &str, _tagid: u32,
        _user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
        mut conn: Connection<DB>, _company: &Company,
        well: &Well, tag: &CustomTag) -> (Status, (ContentType, &'static str)) {
    let q = "DELETE FROM public.custom_tags WHERE well = $1 and id = $2";
    let n: u64 = sqlx::query(q).bind(&well.uuid).bind(&tag.id)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, ""))
    } else {
        (Status::Conflict, (ContentType::Plain, "Error. Tag does not exist."))
    }
}

#[get("/companies/<companyid>/wells/<wellid>/tags/<_tagid>/edit", format = "text/html")]
async fn edit_tag_page(companyid: &str, wellid: &str, _tagid: u32,
                      user: &User, _cwr: &CanWebRead,
                      company: &Company,
                      well: &Well, tag: &CustomTag) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/tags";
    let context = context!{user: user, company: company, well: well, tag: tag,
                           is_new_mode: false, back_url: &back_url};
    Template:: render("top/tag-form", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/tags/<tagid>/edit", format = "text/html", rank = 2)]
async fn edit_tag_page_redirect(companyid: &str, wellid: &str, tagid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/tags/" + tagid + "/edit")
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/edit",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_edit_tag(_companyid: &str, _wellid: &str, _tagid: u32,
        _user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
        mut conn: Connection<DB>, _company: &Company, well: &Well, tag: &CustomTag,
        form: Form<Result<Strict<EditTagForm>, Errors<'_>>>) -> (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<EditTagForm> = form.as_ref().unwrap();
    let mut tx: Transaction<'_, Postgres> = conn.begin().await.unwrap();
    let q1 = "UPDATE public.custom_tags SET units_text = $3 WHERE well = $1 and id = $2";
    let q1res = sqlx::query(q1).bind(&well.uuid).bind(&tag.id)
        .bind(&form.units_text).execute(&mut *tx).await;
    if q1res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "edit_tag: server error, q1a.".to_string()))
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "edit_tag: server error, q1b.".to_string()))
        }
    }
    if(tag.name != form.name) || (tag.description != form.description) {
        let q2 = "UPDATE public.custom_tags SET name = $3, description = $4 ".to_owned() +
            "WHERE well = $1 and id = $2";
        let q2res = sqlx::query(&q2).bind(&well.uuid).bind(&tag.id)
            .bind(&form.name).bind(&form.description)
            .execute(&mut *tx).await;
        if q2res.is_err() {
            let rollback_result = tx.rollback().await;
            if rollback_result.is_ok() {
                return (Status::Conflict, (ContentType::Plain,
                    "Error: tag name must be unique.".to_string()))
            } else {
                return (Status::Conflict, (ContentType::Plain,
                    "edit_tag: server error, q2b.".to_string()))
            }
        }
    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => (Status::Accepted, (ContentType::Plain, "".to_string())),
        Err(_) => (Status::Conflict, (ContentType::Plain,
            "Server error on edit_tag query: commit failed.".to_string())),
    }
}

#[get("/companies/<_companyid>/users", format = "text/html")]
async fn users_page(_companyid: &str, mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead, company: &Company) -> Template {
    let users = get_users(&mut conn, company).await;
    let back_url = "/companies";
    let context = context!{user: user, company: company, users: users,
                           back_url: &back_url};
    Template::render("top/users", &context)
}

#[get("/companies/<companyid>/users", format = "text/html", rank = 2)]
fn users_page_redirect(companyid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid + "/users")
}

#[get("/companies/<companyid>/users/new", format = "text/html")]
async fn new_user_page(companyid: &str,
                       user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
                       company: &Company) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/users";
    let context = context!{user: user, company: company, is_new_mode: true,
                           back_url: &back_url};
    Template:: render("top/user-form", &context)
}

#[get("/companies/<companyid>/users/new", format = "text/html", rank = 2)]
async fn new_user_page_redirect(companyid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/users/new")
}

#[post("/companies/<_companyid>/users/new",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_new_user(_companyid: &str,
        _user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
        mut conn: Connection<DB>, company: &Company,
        form: Form<Result<Strict<NewUserForm<'_>>, Errors<'_>>>) ->
            (Status, (ContentType, String)) {
    if let Err(errors) = &*form { 
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<NewUserForm> = form.as_ref().unwrap();
    if(form.category != UserCategory::User) && (company.id != "geomec") {
        return (Status::Conflict, (ContentType::Plain,
            "Only User category is allowed for other companies.".to_string()))
    }
    let q = "INSERT INTO public.users (id,category,company,".to_owned() +
        "first_name,last_name,pwdhash,email,last_active,web_access," +
        "api_access) VALUES " +
        "($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT DO NOTHING";

    // https://docs.rs/argon2/latest/argon2/
    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();
    // Hash password to PHC string ($argon2id$v=19$...)
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2.hash_password(form.password1.as_bytes(), &salt)
        .unwrap().to_string();
    let n: u64 = sqlx::query(&q).bind(&form.id).bind(&form.category)
        .bind(&company.id).bind(&form.first_name).bind(&form.last_name)
        .bind(&password_hash).bind(&form.email).bind(0).bind(&form.web_access)
        .bind(&form.api_access)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, "".to_string()))
    } else {
        (Status::Conflict, (ContentType::Plain,
            "Error. Some dublication exists (id or name or email).".to_string()))
    }
}

#[get("/companies/<companyid>/users/<xuserid>/edit-password", format = "text/html")]
async fn edit_password_page(companyid: &str, xuserid: &str,
                       mut conn: Connection<DB>,
                       user: &User, _cwr: &CanWebRead,
                       company: &Company) -> Option<Template> {
    let xuser_maybe = get_user(&mut conn, xuserid).await;
    if xuser_maybe.is_some() {
        let xuser: User = xuser_maybe.unwrap();
        if(user.category == UserCategory::Admin) &&
                (xuser.category == UserCategory::Admin) &&
                (user.id != xuser.id) {
            // Error: prohibited to edit passwords for other admins
            return None
        } else if(user.category < UserCategory::Admin) && (user.id != xuser.id) {
            // Error: prohibited to edit passwords for other users
            return None
        }

        let back_url = "/companies/".to_owned() + companyid + "/users";
        let context = context!{user: user, company: company, xuser: &xuser,
                               back_url: &back_url};
        Some(Template:: render("top/edit-password", &context))
    } else {
        None
    }
}

#[get("/companies/<companyid>/users/<xuserid>/edit-password", format = "text/html", rank = 2)]
async fn edit_password_page_redirect(companyid: &str, xuserid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/users/" + xuserid +"/edit-password")
}

#[post("/companies/<_companyid>/users/<xuserid>/edit-password",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_edit_password(_companyid: &str, xuserid: &str,
        user: &User, _cwr: &CanWebRead,
        mut conn: Connection<DB>, _company: &Company,
        form: Form<Result<Strict<EditPasswordForm<'_>>, Errors<'_>>>) ->
            (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<EditPasswordForm> = form.as_ref().unwrap();
    let xuser_maybe = get_user(&mut conn, xuserid).await;
    if xuser_maybe.is_none() {
        return (Status::Conflict, (ContentType::Plain, "Error: user does not exist.".to_string()))
    }
    let xuser: User = xuser_maybe.unwrap();
    if(user.category == UserCategory::Admin) &&
            (xuser.category == UserCategory::Admin) &&
            (user.id != xuser.id) {
        return (Status::Conflict, (ContentType::Plain,
            "Error: prohibited to edit passwords for other admins.".to_string()))
    } else if(user.category < UserCategory::Admin) && (user.id != xuser.id) {
        return (Status::Conflict, (ContentType::Plain,
            "Error: prohibited to edit passwords for other users.".to_string()))
    }

    let q = "UPDATE public.users SET pwdhash = $2 WHERE id = $1";

    // https://docs.rs/argon2/latest/argon2/
    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();
    // Hash password to PHC string ($argon2id$v=19$...)
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2.hash_password(form.password1.as_bytes(), &salt)
        .unwrap().to_string();

    let n: u64 = sqlx::query(&q).bind(&xuser.id).bind(&password_hash)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, "".to_string()))
    } else {
        (Status::Conflict, (ContentType::Plain,
            "Error: user does not exist.".to_string()))
    }
}

#[get("/companies/<companyid>/users/<xuserid>/edit", format = "text/html")]
async fn edit_user_page(companyid: &str, xuserid: &str,
                        mut conn: Connection<DB>,
                        user: &User, _cwr: &CanWebRead,
                        company: &Company) -> Option<Template> {
    let xuser_maybe = get_user(&mut conn, xuserid).await;
    if xuser_maybe.is_none() {
        return None
    }
    let xuser: User = xuser_maybe.unwrap();
    let back_url = "/companies/".to_owned() + companyid + "/users";
    let context = context!{user: user, company: company, is_new_mode: false, xuser: &xuser,
                           back_url: &back_url};
    Some(Template:: render("top/user-form", &context))
}

#[get("/companies/<companyid>/users/<xuserid>/edit", format = "text/html", rank = 2)]
async fn edit_user_page_redirect(companyid: &str, xuserid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/users/" + xuserid +"/edit")
}

#[post("/companies/<_companyid>/users/<xuserid>/edit",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_edit_user(_companyid: &str, xuserid: &str,
        user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
        mut conn: Connection<DB>, _company: &Company,
        form: Form<Result<Strict<EditUserForm<'_>>, Errors<'_>>>) ->
            (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<EditUserForm> = form.as_ref().unwrap();

    let xuser_maybe = get_user(&mut conn, xuserid).await;
    if xuser_maybe.is_none() {
        return (Status::Conflict, (ContentType::Plain, "Error: user does not exist.".to_string()))
    }
    let xuser: User = xuser_maybe.unwrap();
    if(xuser.category == UserCategory::Admin) && (user.id != xuser.id) {
        return (Status::Conflict, (ContentType::Plain,
            "Error: prohibited to edit other admins.".to_string()))
    }

    let mut tx: Transaction<'_, Postgres> = conn.begin().await.unwrap();
    let q1 = "UPDATE public.users SET category = $2, ".to_owned() +
        "web_access = $3, api_access = $4 WHERE id = $1";
    let q1res = sqlx::query(&q1).bind(&xuser.id).bind(&form.category)
        .bind(&form.web_access).bind(&form.api_access)
        .execute(&mut *tx).await;
    if q1res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "edit_user: server error, q1a.".to_string()))
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "edit_user: server error, q1b.".to_string()))
        }
    }
    if xuser.email != form.email {
        let q2 = "UPDATE public.users SET email = $2 WHERE id = $1";
        let q2res = sqlx::query(q2).bind(&xuser.id).bind(&form.email)
            .execute(&mut *tx).await;
        if q2res.is_err() {
            let rollback_result = tx.rollback().await;
            if rollback_result.is_ok() {
                return (Status::Conflict, (ContentType::Plain,
                    "Error: this email already exists. Use another email.".to_string()))
            } else {
                return (Status::Conflict, (ContentType::Plain,
                    "edit_user: server error, q2b.".to_string()))
            }
        }
    }
    if(xuser.first_name != form.first_name) || (xuser.last_name != form.last_name) {
        let q3 = "UPDATE public.users SET first_name = $2, last_name = $3 WHERE id = $1";
        let q3res = sqlx::query(q3).bind(&xuser.id)
            .bind(&form.first_name).bind(&form.last_name)
            .execute(&mut *tx).await;
        if q3res.is_err() {
            let rollback_result = tx.rollback().await;
            if rollback_result.is_ok() {
                return (Status::Conflict, (ContentType::Plain,
                    "Error: this name already exists. Use another name.".to_string()))
            } else {
                return (Status::Conflict, (ContentType::Plain,
                    "edit_user: server error, q3b.".to_string()))
            }
        }
    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => (Status::Accepted, (ContentType::Plain, "".to_string())),
        Err(_) => (Status::Conflict, (ContentType::Plain,
            "Server error on edit_user query: commit failed.".to_string())),
    }
}

#[delete("/companies/<_companyid>/users/<xuserid>", format = "text/plain")]
async fn perform_delete_user(_companyid: &str, xuserid: &str,
        user: &User, _oa: &OnlyAdmin, _cwf: &CanWebFull,
        mut conn: Connection<DB>,
        _company: &Company) -> (Status, (ContentType, String)) {
    let q = "DELETE FROM public.users WHERE id = $1";

    let xuser_maybe = get_user(&mut conn, xuserid).await;
    if xuser_maybe.is_none() {
        return (Status::Conflict, (ContentType::Plain, "Error: user does not exist.".to_string()))
    }
    let xuser: User = xuser_maybe.unwrap();
    if(xuser.category == UserCategory::Admin) && (user.id != xuser.id) {
        return (Status::Conflict, (ContentType::Plain,
            "Error: prohibited to delete other admins.".to_string()))
    }

    let n: u64 = sqlx::query(q).bind(&xuser.id)
        .execute(&mut *conn).await.unwrap().rows_affected();
    if n == 1 {
        (Status::Accepted, (ContentType::Plain, "".to_string()))
    } else {
        (Status::Conflict, (ContentType::Plain, "Error: user does not exist.".to_string()))
    }
}

#[get("/", format = "text/html")]
fn index_page_redirect1(_user: &User, _cwr: &CanWebRead) -> Redirect {
    Redirect::to(uri!(companies_page))
}

#[get("/",format = "text/html", rank = 2)]
fn index_page_redirect2() -> Redirect {
    Redirect::to(uri!(login_page))
}

#[get("/static-public/<file..>")]
async fn public_files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("web/public/").join(file)).await.ok()
}

#[get("/static-user/<file..>")]
async fn user_files(file: PathBuf, _user: &User,
                    _cwr: &CanWebRead) -> Option<NamedFile> {
    NamedFile::open(Path::new("web/user/").join(file)).await.ok()
}

#[post("/login", format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_login(remote_addr: SocketAddr, mut db: Connection<DB>,
                       mut memdb: Connection<MEMDB>,
                       jar: &CookieJar<'_>, form: Form<Strict<LoginForm>>) -> Status {
    // compare this ip address with redis records
    let remote_addr: &str = &remote_addr.ip().to_string();
    let strikes: Option<u64> = redis::cmd("GET").
        arg(&[remote_addr]).query_async(&mut *memdb).await.unwrap();
    let mut strikes: u64 = match strikes {
        Some(n) => n,
        None => 0
    };
    if strikes >= 10 {
        strikes += 1;
        let strikes_ss = (strikes).to_string();
        redis::cmd("SET").arg(&[&remote_addr, &strikes_ss.as_str(), "EX", "86400"])
            .query_async::<_, ()>(&mut *memdb).await.unwrap();
        let wait_time = time::Duration::from_secs(strikes*3);
        thread::sleep(wait_time);
        return Status::Unauthorized
    }
    //
    let q = "SELECT pwdhash FROM public.users WHERE id = $1";
    let res = sqlx::query(q).bind(&form.username).fetch_optional(&mut *db).await.unwrap();
    let password_hash: String = match res {
        Some(row) => row.get::<&str, &str>("pwdhash").to_string(),
        None => {
            // increment counter, wait some time before return
            strikes += 1;
            let strikes_ss = (strikes).to_string();
            redis::cmd("SET").arg(&[&remote_addr, &strikes_ss.as_str(), "EX", "86400"])
                .query_async::<_, ()>(&mut *memdb).await.unwrap();
            let wait_time = time::Duration::from_secs(strikes*3);
            thread::sleep(wait_time);
            return Status::Unauthorized
        }
    };
    let parsed_hash = PasswordHash::new(&password_hash).unwrap();
    if Argon2::default().verify_password(form.password.as_bytes(), &parsed_hash).is_ok() {
        let maybe_existing_token: Option<String> = redis::cmd("GET").arg(&[&form.username])
            .query_async(&mut *memdb).await.unwrap();
        let token: String = match maybe_existing_token {
            Some(existing_token) => existing_token,
            None => {
                thread_rng().sample_iter(&Alphanumeric)
                    .take(64).map(char::from).collect()
            }
        };
        redis::cmd("SET").arg(&[&form.username, &token, "EX", "1800"])
            .query_async::<_, ()>(&mut *memdb).await.unwrap();
        jar.add_private(Cookie::new("token", token));
        jar.add_private(Cookie::new("userid", form.username.to_string()));
        // set counter to 0
        let strikes_ss = 0.to_string();
        redis::cmd("SET").arg(&[&remote_addr, &strikes_ss.as_str(), "EX", "86400"])
            .query_async::<_, ()>(&mut *memdb).await.unwrap();
        //
        return Status::Accepted
    } else {
        // increment counter, wait some time before return
        strikes += 1;
        let strikes_ss = (strikes).to_string();
        redis::cmd("SET").arg(&[&remote_addr, &strikes_ss.as_str(), "EX", "86400"])
            .query_async::<_, ()>(&mut *memdb).await.unwrap();
        let wait_time = time::Duration::from_secs(strikes*3);
        thread::sleep(wait_time);
        return Status::Unauthorized
    }
}

#[get("/logout")]
async fn perform_logout(user: &User, _cwr: &CanWebRead, mut memdb: Connection<MEMDB>) -> Redirect {
    redis::cmd("DEL").arg(&[&user.id])
        .query_async::<_, ()>(&mut *memdb).await.unwrap();
    Redirect::to(uri!(login_page))
}

#[get("/api/ping")]
fn api_ping(_user: &User, _car: &CanApiRead) -> Status {
    Status::Accepted
}

#[derive(FromForm)]
struct LastPointForm {
    units: PresentationUnits,
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/last-point-f32",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn api_last_point_f32(_companyid: &str, _wellid: &str, _tagid: i32,
        _user: &User, _cwf: &CanApiRead,
        mut conn: Connection<DB>, _company: &Company, well: &Well, tag: &TagF32,
        form: Form<Result<Strict<LastPointForm>, Errors<'_>>>) -> Json<Option<PointF32>> {
    if let Err(_errors) = &*form {
        return Json(None)
    }
    let form: &Strict<LastPointForm> = form.as_ref().unwrap();
    Json(get_last_point_f32(&mut conn, well, tag, &form.units).await)
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/last-point-f64",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn api_last_point_f64(_companyid: &str, _wellid: &str, _tagid: i32,
        _user: &User, _cwf: &CanApiRead,
        mut conn: Connection<DB>, _company: &Company, well: &Well, tag: &TagF64,
        form: Form<Result<Strict<LastPointForm>, Errors<'_>>>) -> Json<Option<PointF64>> {
    if let Err(_errors) = &*form {
        return Json(None)
    }
    let form: &Strict<LastPointForm> = form.as_ref().unwrap();
    Json(get_last_point_f64(&mut conn, well, tag, &form.units).await)
}

#[get("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/is-f32")]
async fn api_is_custom_tag_f32(_companyid: &str, _wellid: &str, _tagid: u32,
        _user: &User, _cwf: &CanApiRead,
        _company: &Company, _well: &Well, tag: &CustomTag) -> Json<bool> {
    if tag.value_size == PointValueSize::F32 {
        Json(true)
    } else {
        Json(false)
    }
}

#[derive(FromForm)]
struct FirstPointForm {
    units: PresentationUnits,
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/first-point-f32",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn api_first_point_f32(_companyid: &str, _wellid: &str, _tagid: i32,
        _user: &User, _cwf: &CanApiRead,
        mut conn: Connection<DB>, _company: &Company, well: &Well, tag: &TagF32,
        form: Form<Result<Strict<FirstPointForm>, Errors<'_>>>) -> Json<Option<PointF32>> {
    if let Err(_errors) = &*form {
        return Json(None)
    }
    let form: &Strict<FirstPointForm> = form.as_ref().unwrap();
    Json(get_first_point_f32(&mut conn, well, tag, &form.units).await)
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/first-point-f64",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn api_first_point_f64(_companyid: &str, _wellid: &str, _tagid: i32,
        _user: &User, _cwf: &CanApiRead,
        mut conn: Connection<DB>, _company: &Company, well: &Well, tag: &TagF64,
        form: Form<Result<Strict<FirstPointForm>, Errors<'_>>>) -> Json<Option<PointF64>> {
    if let Err(_errors) = &*form {
        return Json(None)
    }
    let form: &Strict<FirstPointForm> = form.as_ref().unwrap();
    Json(get_first_point_f64(&mut conn, well, tag, &form.units).await)
}

#[derive(FromForm)]
struct GetPointsForm {
    units: PresentationUnits,
    #[field(name = "timeFrom")]
    #[field(validate = range(0..))]
    time_from: i32,
    #[field(name = "timeTo")]
    #[field(validate = range(0..))]
    time_to: i32,
    #[field(name = "LBS")]
    lbs: bool,
    #[field(name = "RBS")]
    rbs: bool,
    #[field(name = "IPB")]
    ipb: bool
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/points-f32",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn api_points_f32<'a>(_companyid: &'a str, _wellid: &'a str, _tagid: i32,
        _user: &'a User, _cwf: &'a CanApiRead,
        mut conn: Connection<DB>, _company: &'a Company, well: &'a Well, tag: &'a TagF32,
        form: Form<Result<Strict<GetPointsForm>, Errors<'_>>>) -> TextStream![String + 'a] {
    let form: &Strict<GetPointsForm> = form.as_ref().unwrap();
    let mut form = GetPointsForm{units: form.units, time_from: form.time_from,
        time_to: form.time_to, lbs: form.lbs, rbs: form.rbs, ipb: form.ipb};
    //
    let mut is_break = false;
    // adjust range
    let first_point_maybe = get_first_point_f32(
        &mut conn, well, tag, &PresentationUnits::US).await;
    if first_point_maybe.is_some() {
        let first_point = first_point_maybe.unwrap();
        if first_point.time > form.time_from {
            form.time_from = first_point.time;
            form.lbs = false;
            form.ipb = false;
        } else if first_point.time == form.time_from {
            if(form.lbs == true) && (form.ipb == true) {
                form.lbs = false;
                form.ipb = false;
            } else if (form.lbs == false) && (form.ipb == true) {
                form.ipb = false; 
            }
        }
    } else {
            is_break = true;
    }
    if !is_break {
        let last_point_maybe = get_last_point_f32(&mut conn, well, tag,
            &PresentationUnits::US).await;
        if last_point_maybe.is_some() {
            let last_point = last_point_maybe.unwrap();
            if(last_point.time < form.time_to) && (form.time_to != 0) {
                form.time_to = last_point.time;
                form.rbs = false;
            } else if form.time_to == 0 {
                form.time_to = last_point.time;
                form.rbs = false;
            }
        } else {
            is_break = true;
        }
    }
    if !is_break {
        if form.time_from > form.time_to {
            is_break = true;
        } else if form.time_from == form.time_to {
            if (form.ipb == false) && ((form.lbs == true) || (form.rbs == true)) {
                is_break = true;
            }
        }
    }
    if !is_break && (form.ipb == true) {
        if form.lbs == true {
            let point_f32_before_nonstrict_maybe = get_point_before_nonstrict_f32(
                &mut conn, well, tag, form.time_from, &PresentationUnits::US).await;
            if point_f32_before_nonstrict_maybe.is_some() {
                let point_before = point_f32_before_nonstrict_maybe.unwrap();
                form.time_from = point_before.time;
                form.lbs = false;
                form.ipb = false;
            } else {
                form.ipb = false;
            }
        } else if form.lbs == false {
            let point_f32_before_strict_maybe = get_point_before_strict_f32(
                &mut conn, well, tag, form.time_from, &PresentationUnits::US).await;
            if point_f32_before_strict_maybe.is_some() {
                let point_before = point_f32_before_strict_maybe.unwrap();
                form.time_from = point_before.time;
                form.lbs = false;
                form.ipb = false;
            } else {
                form.ipb = false;
            }
        }
    }
    // here, ipb is always off || (is_break == true)
    let batch_size: i32 = 90_000;
    let mut is_first_batch_done = false;  // special case
    TextStream! {
        while !is_break {
            // time_from = form.time_from
            let mut time_to = form.time_from + batch_size;
            if time_to >= form.time_to {
                is_break = true;
                time_to = form.time_to;
            }
            let points: Vec<PointF32>;
            if is_break && !is_first_batch_done {
                // single batch
                if(form.lbs == true) && (form.rbs == true) {
                    // (a,b)
                    points = get_points_f32_lbsrbs(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else if(form.lbs == true) && (form.rbs == false) {
                    // (a,b]
                    points = get_points_f32_lbsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else if(form.lbs == false) && (form.rbs == true) {
                    // [a,b)
                    points = get_points_f32_lbnsrbs(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else {
                    // [a,b]
                    points = get_points_f32_lbnsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                }
            } else if !is_first_batch_done {
                // first batch
                if form.lbs == true {
                    // (a,b]
                    points = get_points_f32_lbsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else {
                    // [a,b]
                    points = get_points_f32_lbnsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                }
                is_first_batch_done = true;
            } else if is_break && is_first_batch_done {
                // last batch
                if form.rbs == true {
                    // (a,b)
                    points = get_points_f32_lbsrbs(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else {
                    // (a,b]
                    points = get_points_f32_lbsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                }
            } else {
                // (a, b]
                // one of middle batches
                points = get_points_f32_lbsrbns(
                    &mut conn, well, tag, form.time_from, time_to, &form.units).await;
            }
            for elem in &points {
                let j: String = json::to_string(&elem).unwrap() + "\n";
                yield j;
            }

            form.time_from += batch_size;
        }    
    } 
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/points-f64",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn api_points_f64<'a>(_companyid: &'a str, _wellid: &'a str, _tagid: i32,
        _user: &'a User, _cwf: &'a CanApiRead,
        mut conn: Connection<DB>, _company: &'a Company, well: &'a Well, tag: &'a TagF64,
        form: Form<Result<Strict<GetPointsForm>, Errors<'_>>>) -> TextStream![String + 'a] {
    let form: &Strict<GetPointsForm> = form.as_ref().unwrap();
    let mut form = GetPointsForm{units: form.units, time_from: form.time_from,
        time_to: form.time_to, lbs: form.lbs, rbs: form.rbs, ipb: form.ipb};
    //
    let mut is_break = false;
    // adjust range
    let first_point_maybe = get_first_point_f64(
        &mut conn, well, tag, &PresentationUnits::US).await;
    if first_point_maybe.is_some() {
        let first_point = first_point_maybe.unwrap();
        if first_point.time > form.time_from {
            form.time_from = first_point.time;
            form.lbs = false;
            form.ipb = false;
        } else if first_point.time == form.time_from {
            if(form.lbs == true) && (form.ipb == true) {
                form.lbs = false;
                form.ipb = false;
            } else if (form.lbs == false) && (form.ipb == true) {
                form.ipb = false; 
            }
        }
    } else {
            is_break = true;
    }
    if !is_break {
        let last_point_maybe = get_last_point_f64(&mut conn, well, tag,
            &PresentationUnits::US).await;
        if last_point_maybe.is_some() {
            let last_point = last_point_maybe.unwrap();
            if(last_point.time < form.time_to) && (form.time_to != 0) {
                form.time_to = last_point.time;
                form.rbs = false;
            } else if form.time_to == 0 {
                form.time_to = last_point.time;
                form.rbs = false;
            }
        } else {
            is_break = true;
        }
    }
    if !is_break {
        if form.time_from > form.time_to {
            is_break = true;
        } else if form.time_from == form.time_to {
            if (form.ipb == false) && ((form.lbs == true) || (form.rbs == true)) {
                is_break = true;
            }
        }
    }
    if !is_break && (form.ipb == true) {
        if form.lbs == true {
            let point_f64_before_nonstrict_maybe = get_point_before_nonstrict_f64(
                &mut conn, well, tag, form.time_from, &PresentationUnits::US).await;
            if point_f64_before_nonstrict_maybe.is_some() {
                let point_before = point_f64_before_nonstrict_maybe.unwrap();
                form.time_from = point_before.time;
                form.lbs = false;
                form.ipb = false;
            } else {
                form.ipb = false;
            }
        } else if form.lbs == false {
            let point_f64_before_strict_maybe = get_point_before_strict_f64(
                &mut conn, well, tag, form.time_from, &PresentationUnits::US).await;
            if point_f64_before_strict_maybe.is_some() {
                let point_before = point_f64_before_strict_maybe.unwrap();
                form.time_from = point_before.time;
                form.lbs = false;
                form.ipb = false;
            } else {
                form.ipb = false;
            }
        }
    }
    // here, ipb is always off || (is_break == true)
    let batch_size: i32 = 90_000;
    let mut is_first_batch_done = false;  // special case
    TextStream! {
        while !is_break {
            // time_from = form.time_from
            let mut time_to = form.time_from + batch_size;
            if time_to >= form.time_to {
                is_break = true;
                time_to = form.time_to;
            }
            let points: Vec<PointF64>;
            if is_break && !is_first_batch_done {
                // single batch
                if(form.lbs == true) && (form.rbs == true) {
                    // (a,b)
                    points = get_points_f64_lbsrbs(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else if(form.lbs == true) && (form.rbs == false) {
                    // (a,b]
                    points = get_points_f64_lbsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else if(form.lbs == false) && (form.rbs == true) {
                    // [a,b)
                    points = get_points_f64_lbnsrbs(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else {
                    // [a,b]
                    points = get_points_f64_lbnsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                }
            } else if !is_first_batch_done {
                // first batch
                if form.lbs == true {
                    // (a,b]
                    points = get_points_f64_lbsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else {
                    // [a,b]
                    points = get_points_f64_lbnsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                }
                is_first_batch_done = true;
            } else if is_break && is_first_batch_done {
                // last batch
                if form.rbs == true {
                    // (a,b)
                    points = get_points_f64_lbsrbs(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                } else {
                    // (a,b]
                    points = get_points_f64_lbsrbns(
                        &mut conn, well, tag, form.time_from, time_to, &form.units).await;
                }
            } else {
                // (a, b]
                // one of middle batches
                points = get_points_f64_lbsrbns(
                    &mut conn, well, tag, form.time_from, time_to, &form.units).await;
            }
            for elem in &points {
                let j: String = json::to_string(&elem).unwrap() + "\n";
                yield j;
            }

            form.time_from += batch_size;
        }    
    } 
}

#[derive(FromForm)]
struct AppendF32PointsForm {
    units: PresentationUnits,
    #[field(name = "nPoints")]
    #[field(validate = range(..=120_000))]
    n_points: u32,
    points: Vec<PointF32>,
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/points-f32-append",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn api_points_f32_append(_companyid: &str, _wellid: &str, _tagid: u32,
        _user: &User, _cwf: &CanApiFull,
        mut conn: Connection<DB>, _company: &Company, well: &Well, tag: &TagF32,
        form: Form<Result<Strict<AppendF32PointsForm>, Errors<'_>>>) ->
        (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<AppendF32PointsForm> = form.as_ref().unwrap();
    let last_point_maybe = get_last_point_f32(&mut conn, well, tag, &PresentationUnits::US).await;
    let mut last_time: i32;
    if last_point_maybe.is_some() {
        last_time = last_point_maybe.unwrap().time;
    } else {
        last_time = 0;
    }
    let mut points: Vec<PointF32> = Vec::with_capacity(form.n_points as usize);
    if tag.id == 1 {
        if well.bhp_mode != ComputationMode::Client {
            let msg: String = "well ".to_owned() + &well.name +
                " does not receive client data for bhp tag";
            return (Status::Conflict, (ContentType::Plain, msg))
        }
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() && (point.value > 0.0) {
                let converted_value = convert_pressure_f32(
                    point.value, &form.units, &PresentationUnits::US);
                let p = PointF32{time: point.time, value: converted_value};
                points.push(p);
                last_time = point.time;
            }
        }
    } else if tag.id == 2 {
        if well.bht_mode != ComputationMode::Client {
            let msg: String = "well ".to_owned() + &well.name +
                " does not receive client data for bht tag";
            return (Status::Conflict, (ContentType::Plain, msg))
        }
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() && (point.value > 0.0) {
                points.push(*point);
                last_time = point.time;
            }
        }
    } else if tag.id == 3 {
        if well.whp_mode != ComputationMode::Client {
            let msg: String = "well ".to_owned() + &well.name +
                " does not receive client data for whp tag";
            return (Status::Conflict, (ContentType::Plain, msg))
        }
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() && (point.value >= 0.0) {
                let converted_value = convert_pressure_f32(
                    point.value, &form.units, &PresentationUnits::US);
                let p = PointF32{time: point.time, value: converted_value};
                points.push(p);
                last_time = point.time;
            }
        }
    } else if tag.id == 4 {
        if well.rate_mode != ComputationMode::Client {
            let msg: String = "well ".to_owned() + &well.name +
                " does not receive client data for rate tag";
            return (Status::Conflict, (ContentType::Plain, msg))
        }
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() && (point.value >= 0.0) {
                let converted_value = convert_rate(
                    point.value, &form.units, &PresentationUnits::US);
                let p = PointF32{time: point.time, value: converted_value};
                points.push(p);
                last_time = point.time;
            }
        }
    } else if tag.id == 5 {
        if well.rho_mode != ComputationMode::Client {
            let msg: String = "well ".to_owned() + &well.name +
                " does not receive client data for density tag";
            return (Status::Conflict, (ContentType::Plain, msg))
        }
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() && (point.value > 0.0) {
                let converted_value = convert_density(
                    point.value, &form.units, &PresentationUnits::US);
                let p = PointF32{time: point.time, value: converted_value};
                points.push(p);
                last_time = point.time;
            }
        }
    } else if tag.id == 7 {
        if well.ii_mode != ComputationMode::Client {
            let msg: String = "well ".to_owned() + &well.name +
                " does not receive client data for injectivity tag";
            return (Status::Conflict, (ContentType::Plain, msg))
        }
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() && (point.value > 0.0) {
                let converted_value = convert_injectivity(
                    point.value, &form.units, &PresentationUnits::US);
                let p = PointF32{time: point.time, value: converted_value};
                points.push(p);
                last_time = point.time;
            }
        }
    } else {
        // all custom tags implicitly have ComputationMode::Client
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() {
                // all custom tags operate only on client units
                // insert as is
                points.push(*point);
                last_time = point.time;
            }
        }
    }
    let mut detached_connection: PgConnection = conn.into_inner().detach();
    let mut tx: Transaction<'_, Postgres> = detached_connection.begin().await.unwrap();
    let q1res = insert_points_f32(&mut tx, well, tag, &points).await;
    if q1res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "api_points_f32_append: server error, q1a.".to_string()))
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "api_points_f32_append: server error, q1b.".to_string()))
        }
    }
    let q2res = update_computer_needed(&mut tx, well, true).await;
    if q2res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "api_points_f32_append: server error, q2a.".to_string()))
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "api_points_f32_append: server error, q2b.".to_string()))
        }
    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => (Status::Accepted, (ContentType::Plain, "".to_string())),
        Err(_) => (Status::Conflict, (ContentType::Plain,
            "Server error on api_points_f32_append query: commit failed.".to_string())),
    }
}

#[derive(FromForm)]
struct AppendF64PointsForm {
    units: PresentationUnits,
    #[field(name = "nPoints")]
    #[field(validate = range(..=120_000))]
    n_points: u32,
    points: Vec<PointF64>,
}

#[post("/companies/<_companyid>/wells/<_wellid>/tags/<_tagid>/points-f64-append",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn api_points_f64_append(_companyid: &str, _wellid: &str, _tagid: u32,
        _user: &User, _cwf: &CanApiFull,
        mut conn: Connection<DB>, _company: &Company, well: &Well, tag: &TagF64,
        form: Form<Result<Strict<AppendF64PointsForm>, Errors<'_>>>) ->
        (Status, (ContentType, String)) {
    if let Err(errors) = &*form {
        return (Status::Conflict, (ContentType::Plain, errors.to_string()))
    }
    let form: &Strict<AppendF64PointsForm> = form.as_ref().unwrap();
    let last_point_maybe = get_last_point_f64(&mut conn, well, tag, &PresentationUnits::US).await;
    let mut last_time: i32;
    if last_point_maybe.is_some() {
        last_time = last_point_maybe.unwrap().time;
    } else {
        last_time = 0;
    }
    let mut points: Vec<PointF64> = Vec::with_capacity(form.n_points as usize);
    if tag.id == 6 {
        if well.vtot_mode != ComputationMode::Client {
            let msg: String = "well ".to_owned() + &well.name +
                " does not receive client data for TotalVolume tag";
            return (Status::Conflict, (ContentType::Plain, msg))
        }
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() && (point.value >= 0.0) {
                let converted_value = convert_volume_f64(
                    point.value, &form.units, &PresentationUnits::US);
                let p = PointF64{time: point.time, value: converted_value};
                points.push(p);
                last_time = point.time;
            }
        }
    } else {
        // all custom tags implicitly have ComputationMode::Client
        for point in form.points.iter() {
            if(point.time > last_time) && point.value.is_finite() &&
                    !point.value.is_subnormal() {
                // all custom tags operate only on client units
                // insert as is
                points.push(*point);
                last_time = point.time;
            }
        }
    }
    let mut detached_connection: PgConnection = conn.into_inner().detach();
    let mut tx: Transaction<'_, Postgres> = detached_connection.begin().await.unwrap();
    let q1res = insert_points_f64(&mut tx, well, tag, &points).await;
    if q1res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "api_points_f64_append: server error, q1a.".to_string()))
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "api_points_f64_append: server error, q1b.".to_string()))
        }
    }
    let q2res = update_computer_needed(&mut tx, well, true).await;
    if q2res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "api_points_f64_append: server error, q2a.".to_string()))
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "api_points_f64_append: server error, q2b.".to_string()))
        }
    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => (Status::Accepted, (ContentType::Plain, "".to_string())),
        Err(_) => (Status::Conflict, (ContentType::Plain,
            "Server error on api_points_f64_append query: commit failed.".to_string())),
    }
}

#[get("/companies/<companyid>/wells/<_wellid>/cycles", format = "text/html")]
async fn cycles_page(companyid: &str, _wellid: &str,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells";
    let bhp_tag = TagF32{id: 1};
    let reduced_bhp_tag = TagF32{id: -1};
    let rate_tag = TagF32{id: 4};
    let reduced_rate_tag = TagF32{id: -4};
    //
    let line1_time_i32: i32;
    let line1_time: i64;
    {
        let last_cycle_maybe = get_last_cycleinfo(&mut conn, well).await;
        if last_cycle_maybe.is_some() {
            let last_cycle = last_cycle_maybe.unwrap();
            line1_time_i32 = last_cycle.t3;
            line1_time = (last_cycle.t3 as i64) * 1000;
        } else {
            let first_point_maybe = get_first_point_f32(&mut conn, well,
                &reduced_bhp_tag, &PresentationUnits::US).await;
            if first_point_maybe.is_some() {
                line1_time_i32 = first_point_maybe.unwrap().time;
                line1_time = (line1_time_i32 as i64) * 1000;
            } else {
                line1_time_i32 = 0;
                line1_time = 0;
            }
        }
    }
    let line2_time: i64;
    {
        let hour: i64 = 60*60*1000;  // ms
        line2_time = line1_time + 2*hour;
    }
    let line3_time: i64;
    {
        let hour: i64 = 60*60*1000;  // ms
        line3_time = line1_time + 8*hour; 
    }
    let initial_view_time_a: i64;
    let initial_view_time_b: i64;
    {
        let day: i64 = 60*60*24*1000;  // ms
        initial_view_time_a = line1_time - 1*day;
        initial_view_time_b = line1_time + 2*day;
    }

    async fn get_timeline_points_f32(conn: &mut Connection<DB>, well: &Well,
            tag: &TagF32, reduced_tag: &TagF32, ref_time: i32) -> Vec<PointF32> {
        let start_time: i32;
        {
            let first_point_maybe = get_first_point_f32(
                conn, well, reduced_tag, &PresentationUnits::US).await;
            if first_point_maybe.is_some() {
                start_time = first_point_maybe.unwrap().time;
            } else {
                let empty_vec: Vec<PointF32> = Vec::new();
                return empty_vec;
            }
        }
        let end_time: i32;
        {
            let last_point_maybe = get_last_point_f32(
                conn, well, reduced_tag, &PresentationUnits::US).await;
            if last_point_maybe.is_some() {
                end_time = last_point_maybe.unwrap().time;
            } else {
                let empty_vec: Vec<PointF32> = Vec::new();
                return empty_vec;
            }
        }
        let time_a: i32;
        let time_b: i32;
        {
            let hour: i32 = 60*60;  // seconds
            {
                let prelim_time_a = ref_time - 12*hour;
                let point_a_maybe = get_point_before_nonstrict_f32(conn, well,
                    reduced_tag, prelim_time_a, &PresentationUnits::US).await;
                if point_a_maybe.is_some() {
                    time_a = point_a_maybe.unwrap().time;
                } else {
                    time_a = start_time;
                }
            }
            {
                let prelim_time_b = ref_time + 13*12*hour;  // 6.5 days
                let point_b_maybe = get_point_after_nonstrict_f32(conn, well,
                    reduced_tag, prelim_time_b, &PresentationUnits::US).await;
                if point_b_maybe.is_some() {
                    time_b = point_b_maybe.unwrap().time;
                } else {
                    time_b = end_time;
                }
            }
        }
        let mut points: Vec<PointF32> = Vec::new();
        if start_time != time_a {
            points = get_points_f32_lbnsrbs(conn, well, reduced_tag, start_time, time_a,
                &well.units).await;
        }
        {
            let raw_points = get_points_f32_lbnsrbns(
                conn, well, tag, time_a, time_b, &PresentationUnits::US).await;
            let mut filtered = lttb_f32(&raw_points, 3000);
            if well.units != PresentationUnits::US {
                for point in filtered.iter_mut() {
                    *point = convert_point_f32(*point, tag, &PresentationUnits::US, &well.units);
                }
            }
            points.append(&mut filtered);
        }
        if time_b != end_time {
            let mut right_sector = get_points_f32_lbsrbns(conn, well, reduced_tag, time_b,
                end_time, &well.units).await;
            points.append(&mut right_sector);
        }
        points
    }

    // bhp
    let bhp_points = get_timeline_points_f32(&mut conn, well, &bhp_tag, &reduced_bhp_tag,
        line1_time_i32).await;
    let bhp = tera_points_f32_to_json_string(&bhp_points);
    // rate
    let rate_points = get_timeline_points_f32(&mut conn, well, &rate_tag, &reduced_rate_tag,
        line1_time_i32).await;
    let rate = tera_points_f32_to_json_string(&rate_points);
    //
    let is_empty_plot: bool;
    if (bhp_points.len() > 0) && (rate_points.len() > 0) {
        is_empty_plot = false;
    } else {
        is_empty_plot = true;
    }
    //
    let titles = get_text_titles(&well.units);
    let cycleinfos = get_cycleinfos(&mut conn, well).await;
 
    let context = context!{user: user, company: company, well: well,
        back_url: &back_url, line1_time, line2_time, line3_time, bhp, rate,
        initial_view_time_a, initial_view_time_b, titles, is_empty_plot,
        cycleinfos
    };
    Template::render("cycles/cycles", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles", format = "text/html", rank = 2)]
fn cycles_page_redirect(companyid: &str, wellid: &str) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles")
}

#[derive(FromForm)]
struct NewCycleForm {
    #[field(validate = range(1..))]
    #[field(name = "T1")]
    t1: i64, // ms
    #[field(validate = range(1..))]
    #[field(name = "T2")]
    t2: i64, // ms
    #[field(validate = range(1..))]
    #[field(name = "T3")]
    t3: i64, // ms
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/new",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_new_cycle(_companyid: &str, _wellid: &str,
                   mut conn: Connection<DB>,
                   user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well,
        form: Form<Strict<NewCycleForm>>) -> (Status, (ContentType, String)) {
    let last_cycle_maybe = get_last_cycleinfo(&mut conn, well).await;
    if last_cycle_maybe.is_some() {
        let last_cycle = last_cycle_maybe.unwrap();
        if ((last_cycle.t3 as i64) * 1000) != form.t1 {
            return (Status::Conflict, (ContentType::Plain,
                    "Cycles discontinuity spotted. Try again.".to_string()))
        }
    }
    let t1 = ((form.t1 as f64)/1000.0).round() as i32;
    let t2 = ((form.t2 as f64)/1000.0).round() as i32;
    let t3 = ((form.t3 as f64)/1000.0).round() as i32;
    let bhp_tag = TagF32{id: 1};
    let min_bhp = get_min_value_f32_lbnsrbns(&mut conn, well,
        &bhp_tag, t1, t2, &well.units).await;
    let max_bhp = get_max_value_f32_lbnsrbns(&mut conn, well,
        &bhp_tag, t1, t2, &well.units).await;
    let whp_tag = TagF32{id: 3};
    let min_whp = get_min_value_f32_lbnsrbns(&mut conn, well,
        &whp_tag, t1, t2, &well.units).await;
    let max_whp = get_max_value_f32_lbnsrbns(&mut conn, well,
        &whp_tag, t1, t2, &well.units).await;
    let bht_tag = TagF32{id: 2};
    let min_bht = get_min_value_f32_lbnsrbns(&mut conn, well,
        &bht_tag, t1, t2, &well.units).await;
    let max_bht = get_max_value_f32_lbnsrbns(&mut conn, well,
        &bht_tag, t1, t2, &well.units).await;
    let rate_tag = TagF32{id: 4};
    let avg_rate = get_avg_value_f32_lbnsrbns(&mut conn, well,
        &rate_tag, t1, t2, &well.units).await;
    let max_rate = get_max_value_f32_lbnsrbns(&mut conn, well,
        &rate_tag, t1, t2, &well.units).await;
    let rho_tag = TagF32{id: 5};
    let max_rho = get_max_value_f32_lbnsrbns(&mut conn, well,
        &rho_tag, t1, t2, &well.units).await;
    let end_rho: Option<f32>;
    {
        let point_maybe = get_point_before_nonstrict_f32(&mut conn, well,
            &rho_tag, t3, &well.units).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            end_rho = Some(point.value);
        } else {
            end_rho = None;
        }
    }
    let vtot_tag = TagF64{id: 6};
    let total_volume: Option<f64>;
    {
        let point_maybe = get_point_before_nonstrict_f64(&mut conn, well,
            &vtot_tag, t3, &well.units).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            total_volume = Some(point.value);
        } else {
            total_volume = None;
        }
    }
    let batch_volume: Option<f32>;
    {
        let volume_t2_maybe = get_interpolated_value_f64(&mut conn, well, &vtot_tag, t2,
            &well.units).await;
        if volume_t2_maybe.is_some() {
            let volume_t2 = volume_t2_maybe.unwrap();
            let volume_t1_maybe = get_interpolated_value_f64(&mut conn, well, &vtot_tag, t1,
                &well.units).await;
            if volume_t1_maybe.is_some() {
                let volume_t1 = volume_t1_maybe.unwrap();
                batch_volume = Some((volume_t2 - volume_t1) as f32);
            } else {
                batch_volume = Some(volume_t2 as f32);
            }
        } else {
            batch_volume = None;
        }
    }
    let ii_tag = TagF32{id: 7};
    let min_ii = get_min_value_f32_lbnsrbns(&mut conn, well,
        &ii_tag, t1, t2, &well.units).await;
    let avg_ii = get_avg_value_f32_lbnsrbns(&mut conn, well,
        &ii_tag, t1, t2, &well.units).await;
    let max_ii = get_max_value_f32_lbnsrbns(&mut conn, well,
        &ii_tag, t1, t2, &well.units).await;
    let cycle = Cycle{well: well.uuid, id: 0, status: CycleStatus::Uncommitted,
        t1: t1, t2: t2, t3: t3, last_action_by: user.uuid, batch_volume: batch_volume,
        total_volume: total_volume, min_bhp: min_bhp, max_bhp: max_bhp,
        min_whp: min_whp, max_whp: max_whp, min_bht: min_bht, max_bht: max_bht,
        avg_rate: avg_rate, max_rate: max_rate, max_rho: max_rho, end_rho: end_rho,
        min_ii: min_ii, avg_ii: avg_ii, max_ii: max_ii, last_rate: None,
        isip_bhp: None, isip_whp: None, waterhammer_bhp_endto: None,
        waterhammer_whp_endto: None, horner_bhp: None, horner_whp: None,
        horner_bht: None, stiffness: None};
    let q = "INSERT INTO public.cycles VALUES ".to_owned() +
        "($1,(SELECT next_cycleid($1)),$2,$3,$4,$5,$6,$7,$8," +
        "$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21," +
        "NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL) ON CONFLICT DO NOTHING";
    let res = sqlx::query(&q).bind(cycle.well).bind(cycle.status).bind(cycle.t1).bind(cycle.t2)
        .bind(cycle.t3).bind(cycle.last_action_by).bind(cycle.batch_volume)
        .bind(cycle.total_volume).bind(cycle.min_bhp).bind(cycle.max_bhp)
        .bind(cycle.min_whp).bind(cycle.max_whp).bind(cycle.min_bht).bind(cycle.max_bht)
        .bind(cycle.avg_rate).bind(cycle.max_rate).bind(cycle.max_rho)
        .bind(cycle.end_rho).bind(cycle.min_ii).bind(cycle.avg_ii).bind(cycle.max_ii)
        .execute(&mut *conn).await;
    if res.is_ok() {
       let last_cycle_info_maybe = get_last_cycleinfo(&mut conn, well).await;
       if last_cycle_info_maybe.is_some() {
           let cycle_info = last_cycle_info_maybe.unwrap();
           if cycle_info.t1 == cycle.t1 {
              (Status::Accepted, (ContentType::Plain, cycle_info.id.to_string())) 
           } else {
               (Status::Conflict, (ContentType::Plain,
                   "Server error during new cycle creation. Try again. (1a)".to_string()))
           }
       } else {
           (Status::Conflict, (ContentType::Plain,
               "Server error during new cycle creation. Try again. (1b)".to_string()))
       }
    } else {
        (Status::Conflict, (ContentType::Plain,
            "Server error during new cycle creation. Try again. (2a)".to_string()))
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>",
    format = "text/plain")]
async fn perform_delete_cycle(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well, cycle: &Cycle) -> Status {
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    if !is_last_cycle {
        return Status::Conflict;
    }
    let q = "DELETE FROM public.cycles WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>", format = "text/html")]
async fn cycle_info_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let injection_duration_ss: String;
    {
        let seconds = (cycle.t2 - cycle.t1) as u64;
        let duration = time::Duration::new(seconds, 0);
        injection_duration_ss = duration.hhmmss();
    }
    let shutin_duration_ss: String;
    {
        let seconds = (cycle.t3 - cycle.t2) as u64;
        let duration = time::Duration::new(seconds, 0);
        shutin_duration_ss = duration.hhmmss();
    }
    let cycle_duration_ss: String;
    {
        let seconds = (cycle.t3 - cycle.t1) as u64;
        let duration = time::Duration::new(seconds, 0);
        cycle_duration_ss = duration.hhmmss();
    }
    let text_units = get_text_units(&well.units);
    let cycle_page = "cycle-info";
    let xuser = get_user_uuid(&mut conn, cycle.last_action_by).await;
    let context = context!{user: user, company: company, well: well, back_url: &back_url,
        cycle: cycle, injection_duration_ss, shutin_duration_ss, cycle_duration_ss,
        text_units, xuser, cycle_page, is_last_cycle};
    Template::render("cycles/info", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>", format = "text/html", rank = 2)]
fn cycle_info_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string())
}

#[derive(FromForm)]
struct ChangeCycleStatusForm {
    #[field(name = "newStatus")]
    new_status: CycleStatus
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/status",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_change_cycle_status(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle,
        form: Form<Strict<ChangeCycleStatusForm>>) -> Status {
    let q = "UPDATE public.cycles SET status = $3 WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id).bind(form.new_status)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/last-rate-bhp",
    format = "text/html")]
async fn cycle_last_rate_bhp_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-last-rate-bhp";
    let is_bhp = true;  // to differentiate between last_rate_bhp and last_rate_whp
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let time_from = cycle.t2 - 15*60;
    let time_to = cycle.t2 + 20*60;
    let pressure: String;
    {
        let tag = TagF32{id: 1};
        let points = get_points_f32_lbnsrbns(&mut conn, well, &tag, time_from,
            time_to, &well.units).await;
        pressure = tera_points_f32_to_json_string(&points);
    }
    let rate: String;
    {
        let tag = TagF32{id: 4};
        let points = get_points_f32_lbnsrbns(&mut conn, well, &tag, time_from,
            time_to, &well.units).await;
        rate = tera_points_f32_to_json_string(&points);
    }
    let context = context!{user: user, company: company, well: well, back_url: &back_url,
        cycle: cycle, cycle_page, titles, text_units, pressure, rate, is_bhp, is_last_cycle};
    Template::render("cycles/last-rate", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/last-rate-bhp",
    format = "text/html", rank = 2)]
fn cycle_last_rate_bhp_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/last-rate-bhp")
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/last-rate-whp",
    format = "text/html")]
async fn cycle_last_rate_whp_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let cycle_page = "cycle-last-rate-whp";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let is_bhp = false;  // to differentiate between last_rate_bhp and last_rate_whp
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let time_from = cycle.t2 - 15*60;
    let time_to = cycle.t2 + 20*60;
    let pressure: String;
    {
        let tag = TagF32{id: 3};
        let points = get_points_f32_lbnsrbns(&mut conn, well, &tag, time_from,
            time_to, &well.units).await;
        pressure = tera_points_f32_to_json_string(&points);
    }
    let rate: String;
    {
        let tag = TagF32{id: 4};
        let points = get_points_f32_lbnsrbns(&mut conn, well, &tag, time_from,
            time_to, &well.units).await;
        rate = tera_points_f32_to_json_string(&points);
    }
    let context = context!{user: user, company: company, well: well, back_url: &back_url,
        cycle: cycle, cycle_page, titles, text_units, pressure, rate, is_bhp, is_last_cycle};
    Template::render("cycles/last-rate", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/last-rate-whp",
    format = "text/html", rank = 2)]
fn cycle_last_rate_whp_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/last-rate-whp")
}

#[derive(FromForm)]
struct LastRateForm {
    #[field(validate = range(1..))]
    time: i64,
    #[field(validate = validate_nonnegative_finite_f32())]
    value: f32,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/last-rate",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_last_rate(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well, cycle: &Cycle,
        form: Form<Strict<LastRateForm>>) -> Status {
    let q = "UPDATE public.cycles SET last_rate = $3 WHERE well = $1 AND id = $2";
    let last_rate: LastRate;
    {
        let time = ((form.time as f64)/1000.0).round() as i32;
        let value = convert_rate(form.value, &well.units, &PresentationUnits::US);
        last_rate = LastRate{time: time, value: value};
    }
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id).bind(last_rate)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/last-rate",
    format = "text/plain")]
async fn perform_delete_last_rate(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "UPDATE public.cycles SET last_rate = NULL WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[derive(Copy, Clone)]
#[derive(PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(FromFormField)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
enum CycleBhpWhpCategory {
    BHP,
    WHP,
}

#[derive(FromForm)] 
struct IsipForm {
    category: CycleBhpWhpCategory,
    #[field(validate = range(1..))]
    time: i64,
    #[field(validate = validate_nonnegative_finite_f32())]
    y1: f32,
    #[field(validate = validate_nonnegative_finite_f32())]
    y2: f32,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/isip",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_isip(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well, cycle: &Cycle,
        form: Form<Strict<IsipForm>>) -> Status {
    let isip: Isip;
    {
        let time = ((form.time as f64)/1000.0).round() as i32;
        let lower_value: f32;
        let upper_value: f32;
        if form.y1 < form.y2 {
            lower_value = convert_pressure_f32(form.y1, &well.units, &PresentationUnits::US);
            upper_value = convert_pressure_f32(form.y2, &well.units, &PresentationUnits::US);
        } else {
            lower_value = convert_pressure_f32(form.y2, &well.units, &PresentationUnits::US);
            upper_value = convert_pressure_f32(form.y1, &well.units, &PresentationUnits::US);
        }
        isip = Isip{time: time, lower_value: lower_value, upper_value: upper_value};
    }
    if form.category == CycleBhpWhpCategory::BHP {
        if(isip.lower_value == 0.0) || (isip.upper_value == 0.0) {
            return Status::Conflict
        }
    }
    let q: &str;
    if form.category == CycleBhpWhpCategory::BHP {
        q = "UPDATE public.cycles SET isip_bhp = $3 WHERE well = $1 AND id = $2";
    } else {
        q = "UPDATE public.cycles SET isip_whp = $3 WHERE well = $1 AND id = $2";
    }
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id).bind(isip)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/isip-bhp",
    format = "text/plain")]
async fn perform_delete_isip_bhp(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "UPDATE public.cycles SET isip_bhp = NULL WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/isip-whp",
    format = "text/plain")]
async fn perform_delete_isip_whp(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "UPDATE public.cycles SET isip_whp = NULL WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[derive(FromForm)]
struct WaterHammerForm {
    category: CycleBhpWhpCategory,
    #[field(name = "endTime")]
    #[field(validate = range(1..))]
    end_time: i64,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/water-hammer",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_water_hammer(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle,
        form: Form<Strict<WaterHammerForm>>) -> Status {
    let end_to: Option<i32>;
    {
        let time = ((form.end_time as f64)/1000.0).round() as i32;
        end_to = Some(time);
    }
    let q: &str;
    if form.category == CycleBhpWhpCategory::BHP {
        q = "UPDATE public.cycles SET waterhammer_bhp_endto = $3 WHERE well = $1 AND id = $2";
    } else {
        q = "UPDATE public.cycles SET waterhammer_whp_endto = $3 WHERE well = $1 AND id = $2";
    }
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id).bind(end_to)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/water-hammer-bhp",
    format = "text/plain")]
async fn perform_delete_water_hammer_bhp(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "UPDATE public.cycles SET waterhammer_bhp_endto = NULL WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/water-hammer-whp",
    format = "text/plain")]
async fn perform_delete_water_hammer_whp(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "UPDATE public.cycles SET waterhammer_whp_endto = NULL WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[derive(Copy, Clone)]
#[derive(PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(FromFormField)]
#[serde(crate = "rocket::serde")]
enum CycleHornerCategory {
    BHP,
    WHP,
    BHT,
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/horner-bhp",
    format = "text/html")]
async fn cycle_horner_bhp_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-horner-bhp";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let xhorner = CycleHornerCategory::BHP;
    let tag = TagF32{id: 1};
    let horner: String;
    let y_min: Option<f32>;
    {
        let points = get_horner_points(&mut conn, well, cycle, &tag).await;
        horner = tera_points_f64f32_to_json_string(&points);
        let min_val_maybe = get_min_value_f32_lbsrbns(&mut conn, well,
            &tag, cycle.t2, cycle.t3, &well.units).await;
        let max_val_maybe = get_max_value_f32_lbsrbns(&mut conn, well,
            &tag, cycle.t2, cycle.t3, &well.units).await;
        if min_val_maybe.is_some() && max_val_maybe.is_some() {
            let min_val = min_val_maybe.unwrap();
            let max_val = max_val_maybe.unwrap();
            let dy = max_val - min_val;
            y_min = Some(min_val - dy);
        } else {
            y_min = None;
        }
    }
    let context = context!{user: user, company: company, well: well, back_url: &back_url,
        cycle: cycle, cycle_page, titles, text_units, xhorner, horner, y_min, is_last_cycle};
    Template::render("cycles/horner", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/horner-bhp",
    format = "text/html", rank = 2)]
fn cycle_horner_bhp_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/horner-bhp")
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/horner-whp",
    format = "text/html")]
async fn cycle_horner_whp_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-horner-whp";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let xhorner = CycleHornerCategory::WHP;
    let tag = TagF32{id: 3};
    let horner: String;
    let y_min: Option<f32>;
    {
        let points = get_horner_points(&mut conn, well, cycle, &tag).await;
        horner = tera_points_f64f32_to_json_string(&points);
        let min_val_maybe = get_min_value_f32_lbsrbns(&mut conn, well,
            &tag, cycle.t2, cycle.t3, &well.units).await;
        let max_val_maybe = get_max_value_f32_lbsrbns(&mut conn, well,
            &tag, cycle.t2, cycle.t3, &well.units).await;
        if min_val_maybe.is_some() && max_val_maybe.is_some() {
            let min_val = min_val_maybe.unwrap();
            let max_val = max_val_maybe.unwrap();
            let dy = max_val - min_val;
            y_min = Some(min_val - dy);
        } else {
            y_min = None;
        }
    }
    let context = context!{user: user, company: company, well: well, back_url: &back_url,
        cycle: cycle, cycle_page, titles, text_units, xhorner, horner, y_min, is_last_cycle};
    Template::render("cycles/horner", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/horner-whp",
    format = "text/html", rank = 2)]
fn cycle_horner_whp_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/horner-whp")
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/horner-bht",
    format = "text/html")]
async fn cycle_horner_bht_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-horner-bht";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let xhorner = CycleHornerCategory::BHT;
    let tag = TagF32{id: 2};
    let horner: String;
    let y_max: Option<f32>;
    let y_min: Option<f32>;
    {
        let points = get_horner_points(&mut conn, well, cycle, &tag).await;
        horner = tera_points_f64f32_to_json_string(&points);
        let min_val_maybe = get_min_value_f32_lbsrbns(&mut conn, well,
            &tag, cycle.t2, cycle.t3, &well.units).await;
        let max_val_maybe = get_max_value_f32_lbsrbns(&mut conn, well,
            &tag, cycle.t2, cycle.t3, &well.units).await;
        if min_val_maybe.is_some() && max_val_maybe.is_some() {
            let min_val = min_val_maybe.unwrap();
            let max_val = max_val_maybe.unwrap();
            let dy = max_val - min_val;
            y_max = Some(max_val + dy);
            y_min = Some(min_val.floor());
        } else {
            y_max = None;
            y_min = None;
        }
    }
    let context = context!{user: user, company: company, well: well, back_url: &back_url,
        cycle: cycle, cycle_page, titles, text_units, xhorner, horner, y_max, y_min,
        is_last_cycle};
    Template::render("cycles/horner", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/horner-bht",
    format = "text/html", rank = 2)]
fn cycle_horner_bht_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/horner-bht")
}

#[derive(FromForm)]
struct HornerForm {
    category: CycleHornerCategory,
    #[field(validate = validate_finite_f64())]
    x1: f64,
    #[field(validate = validate_nonnegative_finite_f32())]
    y1: f32,
    #[field(validate = neq(self.x1))]
    #[field(validate = validate_positive_finite_f64())]
    x2: f64,
    #[field(validate = validate_nonnegative_finite_f32())]
    y2: f32,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/horner",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_horner(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well, cycle: &Cycle,
        form: Form<Strict<HornerForm>>) -> (Status, (ContentType, String)) {
    if (form.category == CycleHornerCategory::BHP) || (form.category == CycleHornerCategory::BHT) {
        if (form.y1 == 0.0) || (form.y2 == 0.0) {
            return (Status::Conflict, (ContentType::Plain,
                "Both points must be above y=0.".to_string()))
        }
    }
    let horner: Option<Horner>;
    {
        let x1: f64;
        let y1: f32;
        let x2: f64;
        let y2: f32;
        if form.x1 < form.x2 {
            x1 = form.x1;
            if well.units == PresentationUnits::US {
                y1 = form.y1;
            } else {
                if form.category == CycleHornerCategory::BHT {
                    y1 = form.y1;
                } else {
                    y1 = convert_pressure_f32(form.y1, &well.units, &PresentationUnits::US);
                }
            }
            x2 = form.x2;
            if well.units == PresentationUnits::US {
                y2 = form.y2;
            } else {
                if form.category == CycleHornerCategory::BHT {
                    y2 = form.y2;
                } else {
                    y2 = convert_pressure_f32(form.y2, &well.units, &PresentationUnits::US);
                }
            }
        } else {
            x1 = form.x2;
            if well.units == PresentationUnits::US {
                y1 = form.y2;
            } else {
                if form.category == CycleHornerCategory::BHT {
                    y1 = form.y2;
                } else {
                    y1 = convert_pressure_f32(form.y2, &well.units, &PresentationUnits::US);
                }
            }
            x2 = form.x1;
            if well.units == PresentationUnits::US {
                y2 = form.y1;
            } else {
                if form.category == CycleHornerCategory::BHT {
                    y2 = form.y1;
                } else {
                    y2 = convert_pressure_f32(form.y1, &well.units, &PresentationUnits::US);
                }
            }
        }
        let value: f32;
        {
            let k: f64 = ((y2-y1) as f64)/(x2-x1);
            value = ((y1 as f64) - x1*k) as f32;
        }
        if form.category == CycleHornerCategory::WHP {
            if value < 0.0 {
                return (Status::Conflict, (ContentType::Plain,
                    "Horner value must be nonnegative.".to_string()))
            }
        } else {
            if value <= 0.0 {
                return (Status::Conflict, (ContentType::Plain,
                    "Horner value must be positive.".to_string()))
            }
        }
        horner = Some(Horner{value: value, x1: x1, y1: y1, x2: x2, y2: y2});
    }
    let q: &str;
    if form.category == CycleHornerCategory::BHP {
        q = "UPDATE public.cycles SET horner_bhp = $3 WHERE well = $1 AND id = $2";
    } else if form.category == CycleHornerCategory::WHP {
        q = "UPDATE public.cycles SET horner_whp = $3 WHERE well = $1 AND id = $2";
    } else {
        q = "UPDATE public.cycles SET horner_bht = $3 WHERE well = $1 AND id = $2";
    }
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id).bind(horner)
        .execute(&mut *conn).await;
    if res.is_ok() {
       return (Status::Accepted, (ContentType::Plain, "".to_string()))
    } else {
       return (Status::Conflict, (ContentType::Plain,
           "Server error when saving a Horner line.".to_string()))
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/horner-bhp",
    format = "text/plain")]
async fn perform_delete_horner_bhp(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "UPDATE public.cycles SET horner_bhp = NULL WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/horner-whp",
    format = "text/plain")]
async fn perform_delete_horner_whp(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "UPDATE public.cycles SET horner_whp = NULL WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/horner-bht",
    format = "text/plain")]
async fn perform_delete_horner_bht(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "UPDATE public.cycles SET horner_bht = NULL WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/fourier-bhp",
    format = "text/html")]
async fn cycle_fourier_bhp_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-fourier-bhp";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let fourier: String;
    let y_max: Option<f32>;
    let is_bhp = true;
    {
        let tag = TagF32{id: 1};
        if cycle.waterhammer_bhp_endto.is_some() && cycle.isip_bhp.is_some() {
            let time_from = cycle.isip_bhp.unwrap().time;
            let time_to = cycle.waterhammer_bhp_endto.unwrap();
            let points = get_points_f32_lbsrbns(&mut conn, well, &tag,
                time_from, time_to, &PresentationUnits::US).await;
            let spectrum = water_hammer_fft(&points);
            fourier = tera_points_f32f32_to_json_string(&spectrum);
            if spectrum.len() > 0 {
                let mut max_val: f32 = 0.0;
                for elem in spectrum.iter() {
                    if elem.y > max_val {
                        max_val = elem.y;
                    }
                }
                y_max = Some((max_val + 0.5).ceil());
            } else {
                y_max = None;
            }
        } else {
            fourier = "[]".to_string();
            y_max = None;
        }
    }
    let points = get_fourier_points(&mut conn, cycle, &CycleFourierCategory::BHP).await;
    let context = context!{user: user, company: company, well: well, back_url: &back_url,
        cycle: cycle, cycle_page, titles, text_units, fourier, is_bhp, y_max, points,
        is_last_cycle};
    Template::render("cycles/fourier", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/fourier-bhp",
    format = "text/html", rank = 2)]
fn cycle_fourier_bhp_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/fourier-bhp")
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/fourier-whp",
    format = "text/html")]
async fn cycle_fourier_whp_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-fourier-whp";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let fourier: String;
    let y_max: Option<f32>;
    let is_bhp = false;
    {
        let tag = TagF32{id: 3};
        if cycle.waterhammer_whp_endto.is_some() && cycle.isip_whp.is_some() {
            let time_from = cycle.isip_whp.unwrap().time;
            let time_to = cycle.waterhammer_whp_endto.unwrap();
            let points = get_points_f32_lbsrbns(&mut conn, well, &tag,
                time_from, time_to, &PresentationUnits::US).await;
            let spectrum = water_hammer_fft(&points);
            fourier = tera_points_f32f32_to_json_string(&spectrum);
            if spectrum.len() > 0 {
                let mut max_val: f32 = 0.0; 
                for elem in spectrum.iter() {
                    if elem.y > max_val {
                        max_val = elem.y;
                    }
                }
                y_max = Some((max_val + 0.5).ceil());
            } else {
                y_max = None;
            }
        } else {
            fourier = "[]".to_string();
            y_max = None;
        }
    }
    let points = get_fourier_points(&mut conn, cycle, &CycleFourierCategory::WHP).await;
    let context = context!{user: user, company: company, well: well, back_url: &back_url,
        cycle: cycle, cycle_page, titles, text_units, fourier, is_bhp, y_max, points,
        is_last_cycle};
    Template::render("cycles/fourier", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/fourier-whp",
    format = "text/html", rank = 2)]
fn cycle_fourier_whp_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/fourier-whp")
}

#[derive(FromForm)]
struct FourierForm {
    category: CycleFourierCategory,
    points: Vec<PointF32F32>
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/fourier",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_fourier(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well, cycle: &Cycle,
        form: Form<Strict<FourierForm>>) -> Status {
    for point in form.points.iter() {
        if (point.x <= 0.0) || (point.y <= 0.0) {
            return Status::Conflict;
        }
        if !point.x.is_finite() || !point.y.is_finite() {
            return Status::Conflict;
        }
        if point.x.is_subnormal() || point.y.is_subnormal() {
            return Status::Conflict; 
        }
    }
    let n_points = form.points.len();
    if n_points == 0 {
        return Status::Conflict;
    }
    let mut tx: Transaction<'_, Postgres> = conn.begin().await.unwrap();
    let q1: &str;
    if form.category == CycleFourierCategory::BHP {
        q1 = "DELETE FROM public.fourier WHERE well = $1 AND cycle = $2 AND category = 'bhp'";
    } else {
        q1 = "DELETE FROM public.fourier WHERE well = $1 AND cycle = $2 AND category = 'whp'";
    }
    let q1res = sqlx::query(&q1).bind(cycle.well).bind(cycle.id).bind(&form.category)
        .execute(&mut *tx).await;
    if q1res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return Status::Conflict;
        } else {
            return Status::Conflict;
        }
    }
    let mut q2: String;
    {
        q2 = "INSERT INTO public.fourier VALUES ".to_owned();
        let mut idx = 1;
        for point in form.points.iter() {
            let val_ss: String;
            if form.category == CycleFourierCategory::BHP {
              val_ss = format!(
                  "({},{},'bhp',(SELECT next_fourier_bhp_id({}::INT2,{}::INT2)),{},{})",
                  well.uuid, cycle.id, well.uuid, cycle.id, point.x, point.y);
            } else {
              val_ss = format!(
                  "({},{},'whp',(SELECT next_fourier_whp_id({}::INT2,{}::INT2)),{},{})",
                  well.uuid, cycle.id, well.uuid, cycle.id, point.x, point.y);
            }
            q2 += val_ss.as_str();
            if idx != n_points {
                q2 += ", ";
                idx += 1;
            } else {
                q2 += " ON CONFLICT DO NOTHING";
            }
        }
    }
    let q2res = sqlx::query(&q2).execute(&mut *tx).await;
    if q2res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return Status::Conflict;
        } else {
            return Status::Conflict;
        }
    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => Status::Accepted,
        Err(_) => Status::Conflict,
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/fourier-bhp",
    format = "text/plain")]
async fn perform_delete_fourier_bhp(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "DELETE FROM public.fourier WHERE well = $1 AND cycle = $2 AND category = 'bhp'";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/fourier-whp",
    format = "text/plain")]
async fn perform_delete_fourier_whp(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "DELETE FROM public.fourier WHERE well = $1 AND cycle = $2 AND category = 'whp'";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/stiffness-timeshift",
    format = "text/html")]
async fn cycle_stiffness_timeshift_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-stiffness-timeshift";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let bhp: String;
    let rate: String;
    {
        let time_from = cycle.t1 - 15*60;
        let time_to = cycle.t1 + 20*60;
        {
            let tag = TagF32{id: 1};
            let points = get_points_f32_lbnsrbns(&mut conn, well, &tag, time_from,
                time_to, &well.units).await;
            bhp = tera_points_f32_to_json_string(&points);
        }
        {
           let tag = TagF32{id: 4};
            let points = get_points_f32_lbnsrbns(&mut conn, well, &tag, time_from,
                time_to, &well.units).await;
            rate = tera_points_f32_to_json_string(&points);
        }
    }
    let context = context!{user, company, well, back_url: &back_url,
        cycle, cycle_page, titles, text_units, bhp, rate, is_last_cycle};
    Template::render("cycles/stiffness-timeshift", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/stiffness-timeshift",
    format = "text/html", rank = 2)]
fn cycle_stiffness_timeshift_page_redirect(companyid: &str, wellid: &str,
        cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/stiffness-timeshift")
}

#[derive(FromForm)]
struct StiffnessTimeshiftForm {
    #[field(name = "rateTime")]
    #[field(validate = range(1..))]
    rate_time_ms: i64,
    #[field(name = "bhpTime")]
    #[field(validate = range(1..))]
    bhp_time_ms: i64,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/stiffness-timeshift",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_stiffness_timeshift(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle,
        form: Form<Strict<StiffnessTimeshiftForm>>) -> Status {
    let stiffness: Option<Stiffness>;
    {
        let timeshift = (((form.rate_time_ms - form.bhp_time_ms) as f64)/1000.0) as f32;
        stiffness = Some(Stiffness{timeshift: timeshift,
            rate_time_ms: form.rate_time_ms as f64,
            bhp_time_ms: form.bhp_time_ms as f64});
    }
    let q = "UPDATE public.cycles SET stiffness = $3 WHERE well = $1 AND id = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id).bind(stiffness)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/stiffness",
    format = "text/html")]
async fn cycle_stiffness_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-stiffness";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let xdata: String;
    let intersections: Vec<StiffnessIntersection>;
    {
        if cycle.stiffness.is_some() {
            intersections = get_stiffness_intersections(&mut conn, cycle).await;
            let threshold: usize = 3000;
            let bhp: Vec<PointF32>;
            {
                let tag = TagF32{id: 1};
                let points = get_points_f32_lbnsrbns(&mut conn, well, &tag,
                    cycle.t1, cycle.t2, &PresentationUnits::US).await;
                bhp = lttb_f32(&points, threshold);
            }
            let n_bhp = bhp.len();
            let rate: Vec<PointF32>;
            {
                let tag = TagF32{id: 4};
                let points = get_points_f32_lbnsrbns(&mut conn, well, &tag,
                    cycle.t1, cycle.t2, &PresentationUnits::US).await;
                rate = lttb_f32(&points, threshold);

            }
            let n_rate = rate.len();
            if (n_bhp > 2) && (n_rate > 2) {
                let mut points: Vec<PointF32F32> = Vec::with_capacity(n_bhp);
                let timeshift: f32 = cycle.stiffness.unwrap().timeshift;
                let mut rate_idx: usize = 0;
                let first_rate_time = rate[0].time as f64;
                for elem in bhp.iter() {
                    let bhp_time = (elem.time as f64) + (timeshift as f64);
                    if bhp_time < first_rate_time {
                        continue;
                    }
                    // move to recent rate point
                    loop {
                        if rate_idx == n_rate - 1 {
                            break;
                        }
                        if (bhp_time >= rate[rate_idx].time as f64) &&
                                (bhp_time < rate[rate_idx+1].time as f64) {
                            break;
                        }
                        rate_idx += 1;
                    }
                    let rate_value: f32;
                    if rate_idx < n_rate - 1 {
                        let ra = rate[rate_idx];
                        let rb = rate[rate_idx + 1];
                        let dt = (rb.time - ra.time) as f32;
                        let k = (rb.value - ra.value)/dt;
                        rate_value = ra.value + ((bhp_time - ra.time as f64) as f32) * k;
                    } else {
                        rate_value = rate[rate_idx].value;
                    }
                    if rate_value > 0.0 {
                        points.push(PointF32F32{x: rate_value, y: elem.value});
                    }
                }
                if well.units != PresentationUnits::US {
                    for point in points.iter_mut() {
                        point.x = convert_rate(point.x, &PresentationUnits::US, &well.units);
                        point.y = convert_pressure_f32(point.y, &PresentationUnits::US, &well.units);
                    }
                }
                xdata = tera_points_f32f32_to_json_string(&points);
            } else {
                xdata = "[]".to_string();
            }
        } else {
            xdata = "[]".to_string();
            intersections = Vec::new();
        }
    }
    let context = context!{user, company, well, back_url: &back_url,
        cycle, cycle_page, titles, text_units, xdata, intersections, is_last_cycle};
    Template::render("cycles/stiffness", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/stiffness",
    format = "text/html", rank = 2)]
fn cycle_stiffness_page_redirect(companyid: &str, wellid: &str, cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/stiffness")
}

#[derive(FromForm)]
struct StiffnessForm {
    #[field(validate = validate_positive_finite_f32())]
    ax1: f32,
    #[field(validate = validate_positive_finite_f32())]
    ay1: f32,
    #[field(validate = validate_positive_finite_f32())]
    ax2: f32,
    #[field(validate = validate_positive_finite_f32())]
    ay2: f32,
    #[field(validate = validate_positive_finite_f32())]
    bx1: f32,
    #[field(validate = validate_positive_finite_f32())]
    by1: f32,
    #[field(validate = validate_positive_finite_f32())]
    bx2: f32,
    #[field(validate = validate_positive_finite_f32())]
    by2: f32,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/stiffness",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_add_stiffness(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well, cycle: &Cycle,
        form: Form<Strict<StiffnessForm>>) -> Status {
    //
    let line_a: LineF32F32;
    let line_b: LineF32F32;
    {
        if form.ax1 < form.ax2 {
            let x1 = convert_rate(form.ax1, &well.units, &PresentationUnits::US);
            let y1 = convert_pressure_f32(form.ay1, &well.units, &PresentationUnits::US);
            let x2 = convert_rate(form.ax2, &well.units, &PresentationUnits::US);
            let y2 = convert_pressure_f32(form.ay2, &well.units, &PresentationUnits::US);
            line_a = LineF32F32{x1: x1, y1: y1, x2: x2, y2: y2};
        } else {
            let x2 = convert_rate(form.ax1, &well.units, &PresentationUnits::US);
            let y2 = convert_pressure_f32(form.ay1, &well.units, &PresentationUnits::US);
            let x1 = convert_rate(form.ax2, &well.units, &PresentationUnits::US);
            let y1 = convert_pressure_f32(form.ay2, &well.units, &PresentationUnits::US);
            line_a = LineF32F32{x1: x1, y1: y1, x2: x2, y2: y2};
        }
        if form.bx1 < form.bx2 {
            let x1 = convert_rate(form.bx1, &well.units, &PresentationUnits::US);
            let y1 = convert_pressure_f32(form.by1, &well.units, &PresentationUnits::US);
            let x2 = convert_rate(form.bx2, &well.units, &PresentationUnits::US);
            let y2 = convert_pressure_f32(form.by2, &well.units, &PresentationUnits::US);
            line_b = LineF32F32{x1: x1, y1: y1, x2: x2, y2: y2};
        } else {
            let x2 = convert_rate(form.bx1, &well.units, &PresentationUnits::US);
            let y2 = convert_pressure_f32(form.by1, &well.units, &PresentationUnits::US);
            let x1 = convert_rate(form.bx2, &well.units, &PresentationUnits::US);
            let y1 = convert_pressure_f32(form.by2, &well.units, &PresentationUnits::US);
            line_b = LineF32F32{x1: x1, y1: y1, x2: x2, y2: y2};
        }
    }
    let p_frac: f32;
    if (line_a.x1 == line_a.x2) && (line_b.x1 == line_b.x2) {
        // parallel vertical lines
        return Status::Conflict;
    } else if line_a.x1 == line_a.x2 {
        p_frac = line_b.y1 + line_b.k() * (line_a.x1 - line_b.x1);
    } else if line_b.x1 == line_b.x2 {
        p_frac = line_a.y1 + line_a.k() * (line_b.x1 - line_a.x1);
    } else {
        let k_a = line_a.k();
        let k_b = line_b.k();
        if k_a == k_b {
            // parallel nonvertical lines
            return Status::Conflict;
        }
        let b_a = line_a.y1 - k_a * line_a.x1;
        let b_b = line_b.y1 - k_b * line_b.x1;
        /*
         *  find intersection
         *    y = k1 x + b1
         *    y = k2 x + b2
         *    k1 x + b1 = k2 x + b2
         * */
        let x = (b_b - b_a)/(k_a - k_b);
        p_frac = b_a + k_a * x;
    }
    if p_frac < 0.0 {
        // p_frac must be positive
        return Status::Conflict;
    }
    let q: String = "INSERT INTO public.stiffness VALUES ".to_owned() +
        "($1,$2,(SELECT next_stiffness_id($1::INT2,$2::INT2)),$3,$4,$5)";
    let res = sqlx::query(&q).bind(cycle.well).bind(cycle.id).bind(p_frac)
        .bind(line_a).bind(line_b).execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[delete("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/stiffness",
    format = "text/plain")]
async fn perform_delete_stiffness(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, _well: &Well, cycle: &Cycle) -> Status {
    let q = "DELETE FROM public.stiffness WHERE well = $1 AND cycle = $2";
    let res = sqlx::query(q).bind(cycle.well).bind(cycle.id)
        .execute(&mut *conn).await;
    if res.is_ok() {
        Status::Accepted
    } else {
        Status::Conflict
    }
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/hall",
    format = "text/html")]
async fn cycle_hall_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-hall";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let xdata: String;
    {
        let threshold: usize = 1000;
        let bhp: Vec<PointF32>;
        {
            let tag = TagF32{id: 1};
            let points = get_points_f32_lbnsrbns(&mut conn, well, &tag,
                cycle.t1, cycle.t2, &PresentationUnits::US).await;
            bhp = lttb_f32(&points, threshold);
        }
        let n_bhp = bhp.len();
        let rate: Vec<PointF32>;
        {
            let tag = TagF32{id: 4};
            let points = get_points_f32_lbnsrbns(&mut conn, well, &tag,
                cycle.t1, cycle.t2, &PresentationUnits::US).await;
            rate = lttb_f32(&points, threshold);
        }
        let n_rate = rate.len();
        let mut hall: Vec<PointF64F64> = Vec::with_capacity(n_bhp);
        if (n_bhp > 2) && (n_rate > 2) {
            let mut bhp_idx: usize = 0;
            let mut rate_idx: usize = 0;
            let mut hall_idx: usize = 0;
            let first_rate_time = rate[0].time;
            for elem in bhp.iter() {
                if elem.time < first_rate_time {
                    bhp_idx += 1;
                    continue;
                }
                // move to recent rate point
                loop {
                    if rate_idx == n_rate - 1 {
                        break;
                    }
                    if (elem.time >= rate[rate_idx].time) && (elem.time < rate[rate_idx+1].time) {
                        break;
                    }
                    rate_idx += 1;
                }
                let rate_value = rate[rate_idx].value;
                let pressure_integral: f64;
                let volume_integral: f64;
                if hall_idx > 0 {
                    let dt = (elem.time - bhp[bhp_idx-1].time) as f64;
                    pressure_integral = hall[hall_idx-1].y + dt * (elem.value as f64)/86400.0;
                    volume_integral = hall[hall_idx-1].x + dt * (rate_value as f64)/60.0;
                } else {
                    pressure_integral = 0.0;
                    volume_integral = 0.0;
                }
                if hall_idx == 0 {
                    hall.push(PointF64F64{x: volume_integral, y: pressure_integral});
                    hall_idx += 1;
                } else {
                    if volume_integral > hall[hall_idx-1].x {
                        hall.push(PointF64F64{x: volume_integral, y: pressure_integral});
                        hall_idx += 1;
                    }
                }
                bhp_idx += 1;
            }
        }
        if well.units != PresentationUnits::US {
            for point in hall.iter_mut() {
                point.x = convert_volume_f64(point.x, &PresentationUnits::US, &well.units);
                point.y = convert_pressure_f64(point.y, &PresentationUnits::US, &well.units);
            }
        }
        xdata = tera_points_f64f64_to_json_string(&hall);
    }
    let context = context!{user, company, well, back_url: &back_url,
        cycle, cycle_page, titles, text_units, xdata, is_last_cycle};
    Template::render("cycles/hall", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/hall",
    format = "text/html", rank = 2)]
fn cycle_hall_page_redirect(companyid: &str, wellid: &str,
        cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/hall")
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/flow-regimes",
    format = "text/html")]
async fn cycle_flow_regimes_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-flow-regimes";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let context = context!{user, company, well, back_url: &back_url,
        cycle, cycle_page, titles, text_units, is_last_cycle};
    Template::render("cycles/flow-regimes", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/flow-regimes",
    format = "text/html", rank = 2)]
fn cycle_flow_regimes_page_redirect(companyid: &str, wellid: &str,
        cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/flow-regimes")
}

#[derive(FromForm)]
struct CycleSwitcherForm {
    #[field(validate = len(1..200))]
    url: String,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/next-analysis",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn cycle_next_analysis(_companyid: &str, _wellid: &str, _cycleid: u16,
        mut _conn: Connection<DB>, _user: &User, _cwr: &CanWebRead, 
        _company: &Company, _well: &Well, cycle: &Cycle,
         form: Form<Strict<CycleSwitcherForm>>) -> (Status, (ContentType, String)) {
    let url = form.url.as_str();
    let cycle_base: &str;
    let cycle_end: &str;
    {
        let url_len = url.len();
        let index = url.find("/cycles/");
        if index.is_some() {
           let cycle_id_ss = format!("{}", cycle.id);
           let index_split_at = index.unwrap() + "/cycles/".len() + cycle_id_ss.len();
           if index_split_at == 0 || index_split_at > url_len {
               return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
           } else if index_split_at == url_len {
               cycle_base = url;
               cycle_end = "";
           } else {
               let (first, last) = url.split_at(index_split_at); 
               cycle_base = first;
               cycle_end = last;
           }
        } else {
            return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
        }
    }
    let cycle_info_end: String = format!("/cycles/{}", cycle.id);
    let new_url: String;
    if url.ends_with(&cycle_info_end) {
        new_url = cycle_base.to_owned() + "/last-rate-bhp";
    } else if cycle_end == "/last-rate-bhp" {
        new_url = cycle_base.to_owned() + "/last-rate-whp";
    } else if cycle_end == "/last-rate-whp" {
        new_url = cycle_base.to_owned() + "/horner-bhp";
    } else if cycle_end == "/horner-bhp" {
        new_url = cycle_base.to_owned() + "/horner-whp";
    } else if cycle_end == "/horner-whp" {
        new_url = cycle_base.to_owned() + "/horner-bht";
    } else if cycle_end == "/horner-bht" {
        new_url = cycle_base.to_owned() + "/fourier-bhp";
    } else if cycle_end == "/fourier-bhp" {
        new_url = cycle_base.to_owned() + "/fourier-whp";
    } else if cycle_end == "/fourier-whp" {
        new_url = cycle_base.to_owned() + "/stiffness-timeshift";
    } else if cycle_end == "/stiffness-timeshift" {
        new_url = cycle_base.to_owned() + "/stiffness";
    } else if cycle_end == "/stiffness" {
        new_url = cycle_base.to_owned() + "/hall";
    } else if cycle_end == "/hall" {
        new_url = cycle_base.to_owned() + "/flow-regimes";
    } else {
        return (Status::Conflict, (ContentType::Plain,
            "Last analysis already".to_string()))
    }
    (Status::Accepted, (ContentType::Plain, new_url))
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/previous-analysis",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn cycle_previous_analysis(_companyid: &str, _wellid: &str, _cycleid: u16,
        mut _conn: Connection<DB>, _user: &User, _cwr: &CanWebRead,
        _company: &Company, _well: &Well, cycle: &Cycle,
         form: Form<Strict<CycleSwitcherForm>>) -> (Status, (ContentType, String)) {
    let url = form.url.as_str();
    let cycle_base: &str;
    let cycle_end: &str;
    {
        let url_len = url.len();
        let index = url.find("/cycles/");
        if index.is_some() { 
           let cycle_id_ss = format!("{}", cycle.id);
           let index_split_at = index.unwrap() + "/cycles/".len() + cycle_id_ss.len();
           if index_split_at == 0 || index_split_at > url_len {
               return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
           } else if index_split_at == url_len {
               cycle_base = url;
               cycle_end = "";
           } else {
               let (first, last) = url.split_at(index_split_at);
               cycle_base = first;
               cycle_end = last;
           }
        } else {
            return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
        }
    }
    let cycle_info_end: String = format!("/cycles/{}", cycle.id);
    let new_url: String;
    if cycle_end == "/flow-regimes" {
        new_url = cycle_base.to_owned() + "/hall";
    } else if cycle_end == "/hall" {
        new_url = cycle_base.to_owned() + "/stiffness";
    } else if cycle_end == "/stiffness" {
        new_url = cycle_base.to_owned() + "/stiffness-timeshift";
    } else if cycle_end == "/stiffness-timeshift" {
        new_url = cycle_base.to_owned() + "/fourier-whp";
    } else if cycle_end == "/fourier-whp" {
        new_url = cycle_base.to_owned() + "/fourier-bhp";
    } else if cycle_end == "/fourier-bhp" {
        new_url = cycle_base.to_owned() + "/horner-bht";
    } else if cycle_end == "/horner-bht" {
        new_url = cycle_base.to_owned() + "/horner-whp";
    } else if cycle_end == "/horner-whp" {
        new_url = cycle_base.to_owned() + "/horner-bhp";
    } else if cycle_end == "/horner-bhp" {
        new_url = cycle_base.to_owned() + "/last-rate-whp";
    } else if cycle_end == "/last-rate-whp" {
        new_url = cycle_base.to_owned() + "/last-rate-bhp";
    } else if cycle_end == "/last-rate-bhp" {
        new_url = cycle_base.to_owned();
    } else if url.ends_with(&cycle_info_end) {
        return (Status::Conflict, (ContentType::Plain,
            "First analysis already".to_string()))
    } else {
        return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
    }
    (Status::Accepted, (ContentType::Plain, new_url))
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/next-cycle",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn cycle_next_cycle(_companyid: &str, _wellid: &str, _cycleid: u16,
        mut conn: Connection<DB>, _user: &User, _cwr: &CanWebRead,
        _company: &Company, well: &Well, cycle: &Cycle,
         form: Form<Strict<CycleSwitcherForm>>) -> (Status, (ContentType, String)) {
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    if is_last_cycle {
        return (Status::Conflict, (ContentType::Plain, "Last cycle already".to_string()));
    }
    let url = form.url.as_str();
    let new_url: String;
    {
        let url_len = url.len();
        let index = url.find("/cycles/");
        if index.is_some() {
            let cycle_id_ss = format!("{}", cycle.id);
            let index_split_at_1 = index.unwrap() + "/cycles/".len();
            let index_split_at_2 = index_split_at_1 + cycle_id_ss.len();
            if index_split_at_1 == 0 || index_split_at_2 > url_len {
               return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
            } else {
                let (first, _last_a) = url.split_at(index_split_at_1);
                let new_cycle_id_ss = format!("{}", cycle.id + 1);
                if index_split_at_2 == url_len {
                    new_url = first.to_owned() + &new_cycle_id_ss;
                } else {
                    let (_first_b, last) = url.split_at(index_split_at_2);
                    new_url = first.to_owned() + &new_cycle_id_ss + last;
                }
            }
        } else {
            return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
        }
    }
    (Status::Accepted, (ContentType::Plain, new_url))
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/previous-cycle",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn cycle_previous_cycle(_companyid: &str, _wellid: &str, _cycleid: u16,
        mut _conn: Connection<DB>, _user: &User, _cwr: &CanWebRead,
        _company: &Company, _well: &Well, cycle: &Cycle,
         form: Form<Strict<CycleSwitcherForm>>) -> (Status, (ContentType, String)) {
    let url = form.url.as_str();
    if cycle.id == 1 {
        return (Status::Conflict, (ContentType::Plain, "First cycle already".to_string()));
    }
    let new_url: String;
    {
        let url_len = url.len();
        let index = url.find("/cycles/");
        if index.is_some() { 
            let cycle_id_ss = format!("{}", cycle.id);
            let index_split_at_1 = index.unwrap() + "/cycles/".len();
            let index_split_at_2 = index_split_at_1 + cycle_id_ss.len();
            if index_split_at_1 == 0 || index_split_at_2 > url_len {
               return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
            } else {
                let (first, _last_a) = url.split_at(index_split_at_1);
                let new_cycle_id_ss = format!("{}", cycle.id - 1);
                if index_split_at_2 == url_len {
                    new_url = first.to_owned() + &new_cycle_id_ss; 
                } else {
                    let (_first_b, last) = url.split_at(index_split_at_2);
                    new_url = first.to_owned() + &new_cycle_id_ss + last; 
                }
            }
        } else {
            return (Status::Conflict, (ContentType::Plain, "Bad url".to_string()));
        }
    }
    (Status::Accepted, (ContentType::Plain, new_url))
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/fix-batch-volume",
    format = "text/html")]
async fn cycle_fix_batch_volume_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-fix-batch-volume";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let context = context!{user, company, well, back_url: &back_url,
        cycle, cycle_page, titles, text_units, is_last_cycle};
    Template::render("cycles/fix-batch-volume", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/fix-batch-volume",
    format = "text/html", rank = 2)]
fn cycle_fix_batch_volume_page_redirect(companyid: &str, wellid: &str,
        cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/fix-batch-volume")
}

#[derive(FromForm)]
struct FixBatchVolumeForm {
    #[field(name = "newBatchVolume")]
    #[field(validate = validate_nonnegative_finite_f32())]
    new_batch_volume: f32,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/fix-batch-volume",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_fix_batch_volume(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well, cycle: &Cycle,
        form: Form<Strict<FixBatchVolumeForm>>) ->  (Status, (ContentType, String)) {
    let batch_volume = convert_volume_f32(form.new_batch_volume,
        &well.units, &PresentationUnits::US);
    let mut tx: Transaction<'_, Postgres> = conn.begin().await.unwrap();
    let q1 = "LOCK TABLE public.points_f64 IN ACCESS EXCLUSIVE MODE";
    let q1res = sqlx::query(q1).execute(&mut *tx).await;
    if q1res.is_err() {
        let rollback_result = tx.rollback().await;
        if rollback_result.is_ok() {
            return (Status::Conflict, (ContentType::Plain,
                "perform_fix_batch_volume: error code q1_a".to_string()));
        } else {
            return (Status::Conflict, (ContentType::Plain,
                "perform_fix_batch_volume: error code q1_b".to_string()));
        }
    }
    let tag = TagF64{id: 6};
    {
       // interpolate cycle.t1, cycle.t2 range
       let mut points = get_points_f64_lbnsrbns_tx(
           &mut tx, well, &tag, cycle.t1, cycle.t2, &PresentationUnits::US).await;
       let n_points = points.len();
       if n_points > 2 {
           let start_volume = points[0].value;
           let dv = (batch_volume / ((n_points - 1) as f32)) as f64;
           let mut idx: usize = 0;
           for elem in points.iter_mut() {
               elem.value = start_volume + (idx as f64) * dv;
               idx += 1;
           }
           delete_points_f64_lbnsrbns(&mut tx, well, &tag, cycle.t1, cycle.t2).await;
           let q2res = insert_points_f64(&mut tx, well, &tag, &points).await;
           if q2res.is_err() {
               let rollback_result = tx.rollback().await;
               if rollback_result.is_ok() {
                   return (Status::Conflict, (ContentType::Plain,
                       "perform_fix_batch_volume: error code q2_a".to_string()));
               } else {
                   return (Status::Conflict, (ContentType::Plain,
                       "perform_fix_batch_volume: error code q2_b".to_string()));
               }
           }
       } else {
           let rollback_result = tx.rollback().await;
           if rollback_result.is_ok() {
               return (Status::Conflict, (ContentType::Plain,
                   "perform_fix_batch_volume: error code m1_a".to_string()));
           } else {
               return (Status::Conflict, (ContentType::Plain,
                   "perform_fix_batch_volume: error code m1_b".to_string()));
           }
       }
    }
    let dv: f64;
    {
        // adjust all volumes after t2
        if cycle.batch_volume.is_some() {
            dv = (batch_volume - cycle.batch_volume.unwrap()) as f64;
        } else {
            dv = batch_volume as f64;
        }
        let q = "UPDATE public.points_f64 SET value = value + $4 ".to_owned() +
            "WHERE well = $1 AND tag = $2 AND time > $3";
        sqlx::query(&q).bind(well.uuid).bind(tag.id).bind(cycle.t2).bind(dv)
            .execute(&mut *tx).await.unwrap();
    }
    {
        // update cycles: batch_volume, total_volume
        let last_id: i16;
        {
            let last_cycle_info_maybe = get_last_cycleinfo_tx(&mut tx, well).await;
            if last_cycle_info_maybe.is_none() {
                let rollback_result = tx.rollback().await;
                if rollback_result.is_ok() {
                    return (Status::Conflict, (ContentType::Plain,
                        "perform_fix_batch_volume: error code q4_a".to_string()));
                } else {
                    return (Status::Conflict, (ContentType::Plain,
                        "perform_fix_batch_volume: error code q4_b".to_string()));
                }
            }
            last_id = last_cycle_info_maybe.unwrap().id;
        }
        let mut id = cycle.id;
        loop {
            let cycle_maybe = get_cycle(&mut tx, well, id).await;
            if cycle_maybe.is_some() {
                let xcycle = cycle_maybe.unwrap();
                let xbatch: Option<f32>;
                let xvolume: Option<f64>;
                if xcycle.batch_volume.is_some() {
                    xbatch = Some(xcycle.batch_volume.unwrap() + (dv as f32));
                } else {
                    xbatch = Some(dv as f32);
                }
                if xcycle.total_volume.is_some() {
                    xvolume = Some(xcycle.total_volume.unwrap() + dv);
                } else {
                    xvolume = Some(dv);
                }
                let q = "UPDATE public.cycles SET ".to_owned() +
                    "batch_volume = $3, total_volume = $4 WHERE " +
                    "well = $1 AND id = $2";
                let q5res = sqlx::query(&q).bind(well.uuid).bind(id)
                    .bind(xbatch).bind(xvolume)
                    .execute(&mut *tx).await;
                if q5res.is_err() {
                    let rollback_result = tx.rollback().await;
                    if rollback_result.is_ok() {
                        return (Status::Conflict, (ContentType::Plain,
                            "perform_fix_batch_volume: error code q5_a".to_string()));
                    } else {
                        return (Status::Conflict, (ContentType::Plain,
                            "perform_fix_batch_volume: error code q5_b".to_string()));
                    }
                }
            }
            if id == last_id {
                break;
            }
            id += 1;
        }
    }
    {
        let reduced_tag = TagF64{id: -6};
        delete_points_f64_from_nonstrict(&mut tx, well, &reduced_tag, cycle.t1).await;
    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => (Status::Accepted, (ContentType::Plain, "".to_string())),
        Err(_) => (Status::Conflict, (ContentType::Plain,
            "perform_fix_batch_volume query: commit failed.".to_string())),
    }
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<_cycleid>/fix-well-data",
    format = "text/html")]
async fn cycle_fix_well_data_page(companyid: &str, wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   user: &User, _cwr: &CanWebRead,
                   company: &Company, well: &Well, cycle: &Cycle) -> Template {
    let back_url = "/companies/".to_owned() + companyid + "/wells/" + wellid + "/cycles";
    let is_last_cycle = is_last_cycle(&mut conn, well, cycle).await;
    let cycle_page = "cycle-fix-well-data";
    let titles = get_text_titles(&well.units);
    let text_units = get_text_units(&well.units);
    let custom_tags = get_custom_tags(&mut conn, well).await;
    let context = context!{user, company, well, back_url: &back_url,
        cycle, cycle_page, titles, text_units, is_last_cycle, custom_tags};
    Template::render("cycles/fix-well-data", &context)
}

#[get("/companies/<companyid>/wells/<wellid>/cycles/<cycleid>/fix-well-data",
    format = "text/html", rank = 2)]
fn cycle_fix_well_data_page_redirect(companyid: &str, wellid: &str,
        cycleid: u16) -> Redirect {
    Redirect::to("/login/companies/".to_owned() + companyid +
        "/wells/" + wellid + "/cycles/" + &cycleid.to_string() + "/fix-well-data")
}

#[derive(FromForm)]
struct FixWellDataForm {
    tags: Vec<i16>,
}

#[post("/companies/<_companyid>/wells/<_wellid>/cycles/<_cycleid>/fix-well-data",
    format = "application/x-www-form-urlencoded", data = "<form>")]
async fn perform_fix_well_data(_companyid: &str, _wellid: &str, _cycleid: u16,
                   mut conn: Connection<DB>,
                   _user: &User, _cwf: &CanWebFull,
                   _company: &Company, well: &Well, cycle: &Cycle,
        form: Form<Strict<FixWellDataForm>>) ->  Status {
    let custom_tags = get_custom_tags(&mut conn, well).await;
    let mut tx: Transaction<'_, Postgres> = conn.begin().await.unwrap();
    {
        let q = "LOCK TABLE public.points_f64 IN ACCESS EXCLUSIVE MODE";
        sqlx::query(q).execute(&mut *tx).await.unwrap();
    }
    {
        let q = "LOCK TABLE public.points_f32 IN ACCESS EXCLUSIVE MODE";
        sqlx::query(q).execute(&mut *tx).await.unwrap();
    }
    for id in form.tags.iter() { 
        if *id == 6 {
            let tag = TagF64{id: *id};
            delete_points_f64_from_nonstrict(&mut tx, well, &tag, cycle.t1).await;
            let reduced_tag = TagF64{id: -*id};
            delete_points_f64_from_nonstrict(&mut tx, well, &reduced_tag, cycle.t1).await;
        } else if *id <= 7 {
            let tag = TagF32{id: *id};
            delete_points_f32_from_nonstrict(&mut tx, well, &tag, cycle.t1).await;
            let reduced_tag = TagF32{id: -*id};
            delete_points_f32_from_nonstrict(&mut tx, well, &reduced_tag, cycle.t1).await;
        } else {
           for tag in custom_tags.iter() {
               if tag.id == *id {
                   if tag.value_size == PointValueSize::F32 {
                       let tag = TagF32{id: *id};
                       delete_points_f32_from_nonstrict(&mut tx, well, &tag, cycle.t1).await;
                       let reduced_tag = TagF32{id: -*id};
                       delete_points_f32_from_nonstrict(
                           &mut tx, well, &reduced_tag, cycle.t1).await;
                   } else {
                       let tag = TagF64{id: *id};
                       delete_points_f64_from_nonstrict(&mut tx, well, &tag, cycle.t1).await;
                       let reduced_tag = TagF64{id: -*id};
                       delete_points_f64_from_nonstrict(
                           &mut tx, well, &reduced_tag, cycle.t1).await;
                   }
                   break;
               }
           } 
        }
    };
    {
        let q = "DELETE FROM public.cycles WHERE well = $1 AND id >= $2";
        sqlx::query(q).bind(cycle.well).bind(cycle.id).execute(&mut *tx).await.unwrap();
    }
    {
        let q = "UPDATE public.wells SET computer_needed = $2, computed_to = $3 WHERE id = $1";
        sqlx::query(&q).bind(&well.id).bind(true).bind(cycle.t1-1).execute(&mut *tx).await.unwrap();
    }
    let commit_result = tx.commit().await;
    match commit_result {
        Ok(_) => Status::Accepted,
        Err(_) => Status::Conflict,
    }
}

#[get("/health")]
async fn check_health_status() -> Status {
    Status::Accepted
}

#[catch(404)]
fn not_found(req: &Request) -> Template {
    //format!("Sorry, '{}' is not a valid path.", req.uri())
    let context = context!{uri: req.uri()};
    Template::render("catch404", &context)
}

#[catch(default)]
fn default_catcher(status: Status, req: &Request) -> Template {
    let context = context!{status_code: status.code,
        status_reason: status.reason(),
        uri: req.uri()};
    Template::render("catch_default", &context)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![
              public_files,
              user_files,
              index_page_redirect1, 
              index_page_redirect2,
              login_page,
              login_page_next_url,
              perform_login,
              perform_logout,
              companies_page,
              companies_page_redirect,
              new_company_page,
              new_company_page_redirect,
              perform_add_new_company,
              edit_company_page,
              edit_company_page_redirect,
              perform_edit_company,
              perform_delete_company,
              wells_page,
              wells_page_redirect,
              new_well_page,
              new_well_page_redirect,
              perform_add_new_well,
              edit_well_page,
              edit_well_page_redirect,
              perform_edit_well,
              perform_delete_well,
              tags_page,
              tags_page_redirect,
              new_tag_page,
              new_tag_page_redirect,
              perform_add_new_tag,
              edit_tag_page,
              edit_tag_page_redirect,
              perform_edit_tag,
              perform_delete_tag,
              users_page,
              users_page_redirect,
              new_user_page,
              new_user_page_redirect,
              perform_add_new_user,
              edit_password_page,
              edit_password_page_redirect,
              perform_edit_password,
              edit_user_page,
              edit_user_page_redirect,
              perform_edit_user,
              perform_delete_user,
              api_ping,
              api_last_point_f32,
              api_last_point_f64,
              api_is_custom_tag_f32,
              api_first_point_f32,
              api_first_point_f64,
              api_points_f32,
              api_points_f64,
              api_points_f32_append,
              api_points_f64_append,
              cycles_page,
              cycles_page_redirect,
              perform_add_new_cycle,
              perform_delete_cycle,
              cycle_info_page,
              cycle_info_page_redirect,
              perform_change_cycle_status,
              cycle_last_rate_bhp_page,
              cycle_last_rate_bhp_page_redirect,
              cycle_last_rate_whp_page,
              cycle_last_rate_whp_page_redirect,
              perform_add_last_rate,
              perform_delete_last_rate,
              perform_add_isip,
              perform_delete_isip_bhp,
              perform_delete_isip_whp,
              perform_add_water_hammer,
              perform_delete_water_hammer_bhp,
              perform_delete_water_hammer_whp,
              cycle_horner_bhp_page,
              cycle_horner_bhp_page_redirect,
              cycle_horner_whp_page,
              cycle_horner_whp_page_redirect,
              cycle_horner_bht_page,
              cycle_horner_bht_page_redirect,
              perform_add_horner,
              perform_delete_horner_bhp,
              perform_delete_horner_whp,
              perform_delete_horner_bht,
              cycle_fourier_bhp_page,
              cycle_fourier_bhp_page_redirect,
              cycle_fourier_whp_page,
              cycle_fourier_whp_page_redirect,
              perform_add_fourier,
              perform_delete_fourier_bhp,
              perform_delete_fourier_whp,
              cycle_stiffness_timeshift_page,
              cycle_stiffness_timeshift_page_redirect,
              perform_add_stiffness_timeshift,
              cycle_stiffness_page,
              cycle_stiffness_page_redirect,
              perform_add_stiffness,
              perform_delete_stiffness,
              cycle_hall_page,
              cycle_hall_page_redirect,
              cycle_flow_regimes_page,
              cycle_flow_regimes_page_redirect,
              cycle_next_analysis,
              cycle_previous_analysis,
              cycle_next_cycle,
              cycle_previous_cycle,
              cycle_fix_batch_volume_page,
              cycle_fix_batch_volume_page_redirect,
              perform_fix_batch_volume,
              cycle_fix_well_data_page,
              cycle_fix_well_data_page_redirect,
              perform_fix_well_data,
              check_health_status])
        //.attach(Template::fairing())
        .attach(Template::custom(|engines|{
            engines.tera.register_filter("last_active", tera_last_active);
            engines.tera.register_function("user_category_activator",
                                           tera_user_category_activator);
            engines.tera.register_function("can_delete_user", tera_can_delete_user);
        }))
        .attach(DB::init())
        .attach(MEMDB::init())
        .register("/", catchers![not_found])
        .register("/", catchers![default_catcher])
}

