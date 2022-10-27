use sqlx::{Postgres, Pool, Transaction};

#[derive(PartialEq)]
#[derive(sqlx::Type)]
#[sqlx(type_name = "presentation_units", rename_all = "lowercase")]
pub enum PresentationUnits {
    US,
    EU,
}

#[derive(PartialEq)]
#[derive(sqlx::Type)]
#[sqlx(type_name = "computation_mode", rename_all = "lowercase")]
pub enum ComputationMode {
    Off,
    Client,
    Server,
}

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

pub async fn get_wells(pool: &Pool<Postgres>) -> Vec<Well> {
    let q = "SELECT * FROM public.wells";
    sqlx::query_as::<_, Well>(q).fetch_all(pool).await.unwrap()
}

pub async fn update_computer_needed_pool(pool: &Pool<Postgres>, well: &Well, value: bool) {
    let q = "UPDATE public.wells SET computer_needed = $2 WHERE uuid = $1";
    let res = sqlx::query(q).bind(well.uuid).bind(value).execute(pool).await;
    if res.is_err() {
        panic!("update_computer_needed_pool: query has failed!");
    }
}

pub async fn update_computer_needed(tx: &mut Transaction<'_, Postgres>,
        well: &Well, value: bool) {
    let q = "UPDATE public.wells SET computer_needed = $2 WHERE uuid = $1";
    let res = sqlx::query(q).bind(well.uuid).bind(value).execute(&mut **tx).await;
    if res.is_err() {
        panic!("update_computer_needed_pool: query has failed!");
    }
}

pub async fn update_computed_to(tx: &mut Transaction<'_, Postgres>, well: &Well, time: i32) {
    let q = "UPDATE public.wells SET computed_to = $2 WHERE uuid = $1";
    let res = sqlx::query(q).bind(well.uuid).bind(time).execute(&mut **tx).await;
    if res.is_err() {
        panic!("update_computed_to: query has failed!");
    }
}

