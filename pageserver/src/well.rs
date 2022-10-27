use rocket::serde::{Serialize, Deserialize};
use rocket::FromFormField;
use rocket::form::{self};
use rocket_db_pools::Connection;
use rocket_db_pools::sqlx::{Postgres, Transaction, postgres::PgQueryResult};

use crate::database::DB;
use crate::company::Company;

#[derive(Copy, Clone)]
#[derive(PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(FromFormField)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "presentation_units", rename_all = "lowercase")]
pub enum PresentationUnits {
    US,
    EU,
}

#[derive(PartialEq, PartialOrd)]
#[derive(Serialize, Deserialize)]
#[derive(FromFormField)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "computation_mode", rename_all = "lowercase")]
pub enum ComputationMode {
    Off,
    Client,
    Server,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct Well {
    pub id: String,
    pub uuid: i16,
    pub name: String,
    pub company: String,
    pub initial_reservoir_pressure: Option<f32>,
    pub pressure_sensors_height: Option<f32>,
    pub units: PresentationUnits,
    pub bhp_mode: ComputationMode,
    pub bht_mode: ComputationMode,
    pub whp_mode: ComputationMode,  // 10
    pub rate_mode: ComputationMode,
    pub rho_mode: ComputationMode,
    pub vtot_mode: ComputationMode,
    pub ii_mode: ComputationMode,
    pub computer_needed: bool,
    pub computed_to: i32,           // 16
}

pub async fn update_computer_needed(tx: &mut Transaction<'_, Postgres>, well: &Well, value: bool)
        -> Result<PgQueryResult, sqlx::Error> {
    let q = "UPDATE public.wells SET computer_needed = $2 WHERE id = $1";
     sqlx::query(&q).bind(&well.id).bind(value).execute(&mut **tx).await
}

pub async fn get_wells(conn: &mut Connection<DB>, company: &Company) -> Vec<Well> {
    let q = "SELECT * FROM public.wells WHERE company = $1";
    sqlx::query_as::<_, Well>(q).bind(&company.id)
        .fetch_all(&mut **conn).await.unwrap()
}

pub fn validate_computation_modes<'v>(bhp: &ComputationMode,
        _bht: &ComputationMode, whp: &ComputationMode,
        rate: &ComputationMode, rho: &ComputationMode,
        vtot: &ComputationMode, ii: &ComputationMode,
        pressure_sensors_height: &Option<f32>) -> form::Result<'v, ()> {
    if *bhp == ComputationMode::Server {
        if *whp != ComputationMode::Client {
            return Err(form::Error::validation(
                    "If bhp is computed on Server, then whp must be provided by Client."))?
        } else if pressure_sensors_height.is_none() {
            return Err(form::Error::validation(
                    "If bhp is computed on Server, then vertical distance ".to_owned() +
                    "between pressure sensors must be set."))?
        }
    }
    // BHT - pass
    //WHP - pass
    if (*rate != ComputationMode::Client) && (*vtot == ComputationMode::Server) {
            return Err(form::Error::validation(
                    "If rate is not provided by Client, ".to_owned() +
                    "then vtot can not be computed on Server."))?
    }
    if *rho == ComputationMode::Server {
        if pressure_sensors_height.is_none() {
            return Err(form::Error::validation(
                    "If rho is computed on Server, then ".to_owned() +
                    "vertical distance between pressure sensors must be set."))?
        } else if (*bhp == ComputationMode::Off) || (*whp == ComputationMode::Off) ||
                (*rate == ComputationMode::Off) {
            return Err(form::Error::validation(
                    "If rho is computed on Server, then bhp,whp,rate must be enabled."))?
        }
    }
    // VTOT pass
    if *ii == ComputationMode::Server {
        if (*bhp == ComputationMode::Off) || (*rate == ComputationMode::Off) {
            return Err(form::Error::validation(
                    "If injectivity is computed on Server, then bhp, rate must be enabled."))?
        }
    }
    Ok(())
}

