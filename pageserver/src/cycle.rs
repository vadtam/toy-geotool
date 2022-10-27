use rocket::serde::{Serialize, Deserialize};
use rocket::FromFormField;
use rocket_db_pools::Connection;
use rocket_db_pools::sqlx::{Postgres, Transaction};

use crate::database::DB;
use crate::well::{Well,PresentationUnits};
use crate::point::*;
use crate::lttb::*;
use crate::tag::*;

#[derive(Copy, Clone)]
#[derive(PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(FromFormField)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "cycle_status", rename_all = "lowercase")]
pub enum CycleStatus {
    Uncommitted,
    BadData,
    Committed,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "cycle_last_rate")]
pub struct LastRate {
    pub time: i32,
    pub value: f32,
}

#[derive(Copy, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "cycle_isip")]
pub struct Isip {
    pub time: i32,
    pub lower_value: f32,
    pub upper_value: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "cycle_horner")]
pub struct Horner {
    pub value: f32,
    pub x1: f64,  // NB: this is a log10 value
    pub y1: f32,
    pub x2: f64,  // NB: this is a log10 value
    pub y2: f32,
}

#[derive(Copy, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "cycle_stiffness")]
pub struct Stiffness {
  pub timeshift: f32,  // NB: seconds, time(tag Rate) - time(tag BHP)
  pub rate_time_ms: f64,
  pub bhp_time_ms: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct Cycle {
    pub well: i16,  // #1
    pub id: i16,
    pub status: CycleStatus,
    pub t1: i32,  // injection start
    pub t2: i32,  // injection end
    pub t3: i32,  // shutin end
    pub last_action_by: i16,  // user FK
    pub batch_volume: Option<f32>,  // VTOT(t2)-VTOT(t1)
    pub total_volume: Option<f64>,  // total injected volume at t3
    pub min_bhp: Option<f32>,   // [t1,t2]  #10
    pub max_bhp: Option<f32>,   // [t1,t2]
    pub min_whp: Option<f32>,   // [t1,t2]
    pub max_whp: Option<f32>,   // [t1,t2]
    pub min_bht: Option<f32>,   // [t1,t2]
    pub max_bht: Option<f32>,   // [t1,t2]
    pub avg_rate: Option<f32>,  // [t1,t2]
    pub max_rate: Option<f32>,  // [t1,t2]
    pub max_rho: Option<f32>,   // [t1,t2]
    pub end_rho: Option<f32>,   // at t3
    pub min_ii: Option<f32>,  // [t1,t2]  #20
    pub avg_ii: Option<f32>,  // [t1,t2]
    pub max_ii: Option<f32>,  // [t1,t2]
    pub last_rate: Option<LastRate>,
    pub isip_bhp: Option<Isip>,
    pub isip_whp: Option<Isip>,
    pub waterhammer_bhp_endto: Option<i32>,
    pub waterhammer_whp_endto: Option<i32>,
    pub horner_bhp: Option<Horner>,
    pub horner_whp: Option<Horner>,
    pub horner_bht: Option<Horner>,  // #30
    pub stiffness: Option<Stiffness>,  // stiffness timeshift
}

pub fn convert_cycle(mut cycle: Cycle, units_from: &PresentationUnits,
        units_to: &PresentationUnits) -> Cycle {
    if cycle.batch_volume.is_some() {
        let value = cycle.batch_volume.unwrap();
        cycle.batch_volume = Some(convert_volume_f32(value, units_from, units_to));
    }
    if cycle.total_volume.is_some() {
        let value = cycle.total_volume.unwrap();
        cycle.total_volume = Some(convert_volume_f64(value, units_from, units_to));
    }
    if cycle.min_bhp.is_some() {
        let value = cycle.min_bhp.unwrap();
        cycle.min_bhp = Some(convert_pressure_f32(value, units_from, units_to));
    }
    if cycle.max_bhp.is_some() {
        let value = cycle.max_bhp.unwrap();
        cycle.max_bhp = Some(convert_pressure_f32(value, units_from, units_to));
    }
    if cycle.min_whp.is_some() {
        let value = cycle.min_whp.unwrap();
        cycle.min_whp = Some(convert_pressure_f32(value, units_from, units_to));
    }
    if cycle.max_whp.is_some() {
        let value = cycle.max_whp.unwrap();
        cycle.max_whp = Some(convert_pressure_f32(value, units_from, units_to));
    }
    if cycle.avg_rate.is_some() {
        let value = cycle.avg_rate.unwrap();
        cycle.avg_rate = Some(convert_rate(value, units_from, units_to));
    }
    if cycle.max_rate.is_some() {
        let value = cycle.max_rate.unwrap();
        cycle.max_rate = Some(convert_rate(value, units_from, units_to));
    }
    if cycle.max_rho.is_some() {
        let value = cycle.max_rho.unwrap();
        cycle.max_rho = Some(convert_density(value, units_from, units_to));
    }
    if cycle.end_rho.is_some() {
        let value = cycle.end_rho.unwrap();
        cycle.end_rho = Some(convert_density(value, units_from, units_to));
    }
    if cycle.min_ii.is_some() {
        let value = cycle.min_ii.unwrap();
        cycle.min_ii = Some(convert_injectivity(value, units_from, units_to));
    }
    if cycle.avg_ii.is_some() {
        let value = cycle.avg_ii.unwrap();
        cycle.avg_ii = Some(convert_injectivity(value, units_from, units_to));
    }
    if cycle.max_ii.is_some() {
        let value = cycle.max_ii.unwrap();
        cycle.max_ii = Some(convert_injectivity(value, units_from, units_to));
    }
    if cycle.last_rate.is_some() {
        let mut last_rate = cycle.last_rate.unwrap();
        last_rate.value = convert_rate(last_rate.value, units_from, units_to);
        cycle.last_rate = Some(last_rate);
    }
    if cycle.isip_bhp.is_some() {
        let mut isip = cycle.isip_bhp.unwrap();
        isip.lower_value = convert_pressure_f32(isip.lower_value, units_from, units_to);
        isip.upper_value = convert_pressure_f32(isip.upper_value, units_from, units_to);
        cycle.isip_bhp = Some(isip);
    }
    if cycle.isip_whp.is_some() {
        let mut isip = cycle.isip_whp.unwrap();
        isip.lower_value = convert_pressure_f32(isip.lower_value, units_from, units_to);
        isip.upper_value = convert_pressure_f32(isip.upper_value, units_from, units_to);
        cycle.isip_whp = Some(isip);
    }
    if cycle.horner_bhp.is_some() {
        let mut horner = cycle.horner_bhp.unwrap();
        horner.value = convert_pressure_f32(horner.value, units_from, units_to);
        horner.y1 = convert_pressure_f32(horner.y1, units_from, units_to);
        horner.y2 = convert_pressure_f32(horner.y2, units_from, units_to);
        cycle.horner_bhp = Some(horner);
    }
    if cycle.horner_whp.is_some() {
        let mut horner = cycle.horner_whp.unwrap();
        horner.value = convert_pressure_f32(horner.value, units_from, units_to);
        horner.y1 = convert_pressure_f32(horner.y1, units_from, units_to);
        horner.y2 = convert_pressure_f32(horner.y2, units_from, units_to);
        cycle.horner_whp = Some(horner);
    }
    cycle
}

pub async fn get_cycle(tx: &mut Transaction<'_, Postgres>, well: &Well, cycleid: i16)
        -> Option<Cycle> {
    let q = "SELECT * FROM public.cycles WHERE well = $1 AND id = $2";
    let cycle_maybe = sqlx::query_as::<_, Cycle>(q).bind(well.uuid).bind(cycleid)
        .fetch_optional(&mut **tx).await.unwrap();
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

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct CycleInfo {
    pub id: i16,
    pub status: CycleStatus,
    pub t1: i32,  // injection start
    pub t2: i32,  // injection end
    pub t3: i32,  // shutin end
}

pub async fn get_last_cycleinfo(conn: &mut Connection<DB>, well: &Well) -> Option<CycleInfo> {
    let q = "SELECT id,status,t1,t2,t3 FROM public.cycles ".to_owned() +
        "WHERE well = $1 ORDER BY id DESC LIMIT 1";
    sqlx::query_as::<_, CycleInfo>(&q).bind(well.uuid)
        .fetch_optional(&mut **conn).await.unwrap()
}

pub async fn get_last_cycleinfo_tx(tx: &mut Transaction<'_, Postgres>,
        well: &Well) -> Option<CycleInfo> {
    let q = "SELECT id,status,t1,t2,t3 FROM public.cycles ".to_owned() +
        "WHERE well = $1 ORDER BY id DESC LIMIT 1";
    sqlx::query_as::<_, CycleInfo>(&q).bind(well.uuid)
        .fetch_optional(&mut **tx).await.unwrap()
}

pub async fn is_last_cycle(conn: &mut Connection<DB>, well: &Well, cycle: &Cycle) -> bool {
    let cycle_info_maybe = get_last_cycleinfo(conn, well).await;
    if cycle_info_maybe.is_none() {
        false
    } else {
        let cycle_info = cycle_info_maybe.unwrap();
        if cycle_info.id == cycle.id {
            true
        } else {
            false
        }
    }
}

pub async fn get_cycleinfos(conn: &mut Connection<DB>, well: &Well) -> Vec<CycleInfo> {
    let q = "SELECT id,status,t1,t2,t3 FROM public.cycles ".to_owned() +
        "WHERE well = $1 ORDER BY id ASC";
    sqlx::query_as::<_, CycleInfo>(&q).bind(well.uuid)
        .fetch_all(&mut **conn).await.unwrap()
}

pub async fn get_horner_points(conn: &mut Connection<DB>, well: &Well, cycle: &Cycle,
        tag: &TagF32) -> Vec<PointF64F32> {
    let points: Vec<PointF32>;
    {
        let raw = get_points_f32_lbsrbns(conn, well, tag, cycle.t2, cycle.t3,
            &PresentationUnits::US).await;
        points = lttb_f32(&raw, 1000);
    }
    let mut horner: Vec<PointF64F32> = Vec::with_capacity(points.len());
    let t = (cycle.t2 - cycle.t1) as f64;
    for point in points.iter() {
        let dt = (point.time - cycle.t2) as f64;
        let x: f64 = ((t + dt)/dt ).log10();
        let y: f32;
        {
            if well.units == PresentationUnits::US {
                y = point.value;
            } else {
                if tag.id == 2 {
                    y = point.value;
                } else {
                    y = convert_pressure_f32(point.value, &PresentationUnits::US, &well.units);
                }
            }
        }
        horner.push(PointF64F32{x: x, y: y});
    }
    horner
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct FourierPoint {
    pub id: i16,
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone)]
#[derive(PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(FromFormField)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "fourier_category", rename_all = "lowercase")]
pub enum CycleFourierCategory {
    BHP,
    WHP,
}

pub async fn get_fourier_points(conn: &mut Connection<DB>, cycle: &Cycle,
        category: &CycleFourierCategory) -> Vec<FourierPoint> {
    let q: String;
    if *category == CycleFourierCategory::BHP {
        q = "SELECT id,x,y FROM public.fourier ".to_owned() +
            "WHERE well = $1 AND cycle = $2 AND category = 'bhp'";
    } else {
        q = "SELECT id,x,y FROM public.fourier ".to_owned() +
            "WHERE well = $1 AND cycle = $2 AND category = 'whp'";
    }
    sqlx::query_as::<_, FourierPoint>(&q).bind(cycle.well).bind(cycle.id)
        .fetch_all(&mut **conn).await.unwrap()
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct StiffnessIntersection {
    pub value: f32,
    pub line_a: LineF32F32,
    pub line_b: LineF32F32,
}

pub async fn get_stiffness_intersections(conn: &mut Connection<DB>, cycle: &Cycle)
        -> Vec<StiffnessIntersection> {
    let q = "SELECT value,line_a,line_b FROM public.stiffness ".to_owned() +
        "WHERE well = $1 AND cycle = $2";
    sqlx::query_as::<_, StiffnessIntersection>(&q).bind(cycle.well).bind(cycle.id)
        .fetch_all(&mut **conn).await.unwrap()
}


