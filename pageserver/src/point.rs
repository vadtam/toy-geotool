use rocket::serde::{Serialize, Deserialize};
use rocket_db_pools::Connection;
use rocket_db_pools::sqlx::{Postgres, Transaction, postgres::PgQueryResult};

use crate::database::DB;
use crate::well::{PresentationUnits, Well};
use crate::tag::{TagF32, TagF64};

#[derive(Copy, Clone)]
#[derive(FromForm)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct PointF32 {
    pub time: i32,
    pub value: f32,
}

#[derive(Copy, Clone)]
#[derive(FromForm)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct PointF64 {
    pub time: i32,
    pub value: f64,
}

pub struct PointF64F32 {
    pub x: f64,
    pub y: f32,
}

pub struct PointF64F64 {
    pub x: f64,
    pub y: f64,
}

#[derive(FromForm)]
pub struct PointF32F32 {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "line_f32_f32")]
pub struct LineF32F32 {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl LineF32F32 {
    pub fn k(&self) -> f32 {
        (self.y2 - self.y1)/(self.x2 - self.x1)
    }
}

#[derive(sqlx::FromRow)]
struct ValueF32 {
    pub value: Option<f32>  // sic! to prevent sqlx::error::UnexpectedNullError
}

pub fn convert_pressure_f32(val: f32, units_from: &PresentationUnits,
        units_to: &PresentationUnits) -> f32 {
    if (*units_from == PresentationUnits::US) && (*units_to == PresentationUnits::EU) {
        val / 14.5038
    } else if (*units_from == PresentationUnits::EU) && (*units_to == PresentationUnits::US) {
        val * 14.5038
    } else {
        val
    }
}

pub fn convert_pressure_f64(val: f64, units_from: &PresentationUnits,
        units_to: &PresentationUnits) -> f64 {
    if (*units_from == PresentationUnits::US) && (*units_to == PresentationUnits::EU) {
        val / 14.5038
    } else if (*units_from == PresentationUnits::EU) && (*units_to == PresentationUnits::US) {
        val * 14.5038
    } else {
        val
    }
}

pub fn convert_rate(val: f32, units_from: &PresentationUnits, units_to: &PresentationUnits) -> f32 {
    if (*units_from == PresentationUnits::US) && (*units_to == PresentationUnits::EU) {
        val / 0.10483023333333333
    } else if (*units_from == PresentationUnits::EU) && (*units_to == PresentationUnits::US) {
        // = 6.289814 / 60
        val * 0.10483023333333333
    } else {
        val
    }
}

pub fn convert_density(val: f32, units_from: &PresentationUnits, units_to: &PresentationUnits) -> f32 {
    if (*units_from == PresentationUnits::US) && (*units_to == PresentationUnits::EU) {
        val / 8.33
    } else if (*units_from == PresentationUnits::EU) && (*units_to == PresentationUnits::US) {
        val * 8.33
    } else {
        val
    }
}

pub fn convert_volume_f32(val: f32, units_from: &PresentationUnits, units_to: &PresentationUnits) -> f32 {
    if (*units_from == PresentationUnits::US) && (*units_to == PresentationUnits::EU) {
        val / 6.289814
    } else if (*units_from == PresentationUnits::EU) && (*units_to == PresentationUnits::US) {
        val * 6.289814
    } else {
        val
    }
}

pub fn convert_volume_f64(val: f64, units_from: &PresentationUnits, units_to: &PresentationUnits) -> f64 {
    if (*units_from == PresentationUnits::US) && (*units_to == PresentationUnits::EU) {
        val / 6.289814
    } else if (*units_from == PresentationUnits::EU) && (*units_to == PresentationUnits::US) {
        val * 6.289814
    } else {
        val
    }
}

pub fn convert_injectivity(val: f32, units_from: &PresentationUnits,
        units_to: &PresentationUnits) -> f32 {
    if (*units_from == PresentationUnits::US) && (*units_to == PresentationUnits::EU) {
        val / 10.407999007156745
    } else if (*units_from == PresentationUnits::EU) && (*units_to == PresentationUnits::US) {
        // = 6.289814 bbl * 24 /d  / 14.5038 psi
        val * 10.407999007156745
    } else {
        val
    }
}

pub fn convert_point_f32(mut point: PointF32, tag: &TagF32, units_from: &PresentationUnits,
        units_to: &PresentationUnits) -> PointF32 {
    let tagid_abs: i16 = tag.id.abs();
    if tagid_abs > 7 {
        point
    } else {
        if (tagid_abs == 1) || (tagid_abs == 3) {
            point.value = convert_pressure_f32(point.value, units_from, units_to);
            point
        } else if tagid_abs == 2 {
            point
        } else if tagid_abs == 4 {
            point.value = convert_rate(point.value, units_from, units_to);
            point
        } else if tagid_abs == 5 {
            point.value = convert_density(point.value, units_from, units_to);
            point
        } else if tagid_abs == 6 {
            panic!("convert_point_f32 received tagid.abs(6)")
        } else if tagid_abs == 7 {
            point.value = convert_injectivity(point.value, units_from, units_to);
            point
        } else {
            panic!("convert_point_f32 received tagid 0")
        }
    }
}

pub fn convert_point_f64(mut point: PointF64, tag: &TagF64, units_from: &PresentationUnits,
        units_to: &PresentationUnits) -> PointF64 {
    let tagid_abs: i16 = tag.id.abs();
    if tagid_abs > 7 {
        point
    } else {
        if tagid_abs == 6 {
            point.value = convert_volume_f64(point.value, units_from, units_to);
            point
        } else {
            panic!("convert_point_f64 received bad tagid: {}", tag.id)
        }
    }
}

pub fn convert_point_f32_value(value: f32, tag: &TagF32, units_from: &PresentationUnits,
        units_to: &PresentationUnits) -> f32 {
    let tagid_abs: i16 = tag.id.abs();
    if tagid_abs > 7 {
        value
    } else {
        if (tagid_abs == 1) || (tagid_abs == 3) {
            convert_pressure_f32(value, units_from, units_to)
        } else if tagid_abs == 2 {
            value
        } else if tagid_abs == 4 {
            convert_rate(value, units_from, units_to)
        } else if tagid_abs == 5 {
            convert_density(value, units_from, units_to)
        } else if tagid_abs == 6 {
            panic!("convert_point_f32_value received tagid.abs(6)")
        } else if tagid_abs == 7 {
            convert_injectivity(value, units_from, units_to)
        } else {
            panic!("convert_point_f32_value received tagid 0")
        }
    }
}

pub async fn get_last_point_f32(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, units: &PresentationUnits) -> Option<PointF32> {
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 ORDER BY time DESC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF32 = maybe_point.unwrap();
        let converted_point = convert_point_f32(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_last_point_f64(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, units: &PresentationUnits) -> Option<PointF64> {
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 ORDER BY time DESC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF64 = maybe_point.unwrap();
        let converted_point = convert_point_f64(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_first_point_f32(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, units: &PresentationUnits) -> Option<PointF32> {
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 ORDER BY time ASC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF32 = maybe_point.unwrap();
        let converted_point = convert_point_f32(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_first_point_f64(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, units: &PresentationUnits) -> Option<PointF64> {
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 ORDER BY time ASC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF64 = maybe_point.unwrap();
        let converted_point = convert_point_f64(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn insert_points_f32(tx: &mut Transaction<'_, Postgres>, well: &Well, tag: &TagF32,
        points: &Vec<PointF32>) -> Result<PgQueryResult, sqlx::Error> {
    let len_points: i32 = points.len() as i32;
    if len_points == 0 {
        let pg_query_result: PgQueryResult = Default::default();
        return Ok(pg_query_result);
    }
    let mut q: String = "INSERT INTO public.points_f32 VALUES ".to_owned();
    let mut idx: i32 = 1;
    for point in points.iter() {
        let val_ss: String = format!("({},{},{},{})", well.uuid, tag.id, point.time, point.value);
        q += val_ss.as_str();
        if idx != len_points {
            q += ", ";
            idx += 1;
        } else {
            q += " ON CONFLICT DO NOTHING";
        }
    }
    sqlx::query(&q).execute(&mut **tx).await
}

pub async fn insert_points_f64(tx: &mut Transaction<'_, Postgres>, well: &Well, tag: &TagF64,
        points: &Vec<PointF64>) -> Result<PgQueryResult, sqlx::Error> {
    let len_points: i32 = points.len() as i32;
    if len_points == 0 {
        let pg_query_result: PgQueryResult = Default::default();
        return Ok(pg_query_result);
    }
    let mut q: String = "INSERT INTO public.points_f64 VALUES ".to_owned();
    let mut idx: i32 = 1;
    for point in points.iter() {
        let val_ss: String = format!("({},{},{},{})", well.uuid, tag.id, point.time, point.value);
        q += val_ss.as_str();
        if idx != len_points {
            q += ", ";
            idx += 1;
        } else {
            q += " ON CONFLICT DO NOTHING";
        }
    }
    sqlx::query(&q).execute(&mut **tx).await
}

pub async fn delete_points_f64_lbnsrbns(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: &TagF64, time_from: i32, time_to: i32) {
    // [a,b]
    let q = "DELETE FROM public.points_f64 WHERE well = $1 AND ".to_owned() +
        "tag = $2 AND time >= $3 AND time <= $4";
    sqlx::query(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).execute(&mut **tx).await.unwrap();
}

pub async fn delete_points_f64_from_nonstrict(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: &TagF64, time_from: i32) {
    // [a,Inf)
    let q = "DELETE FROM public.points_f64 WHERE well = $1 AND ".to_owned() +
        "tag = $2 AND time >= $3";
    sqlx::query(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).execute(&mut **tx).await.unwrap();
}

pub async fn delete_points_f32_from_nonstrict(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: &TagF32, time_from: i32) {
    // [a,Inf)
    let q = "DELETE FROM public.points_f32 WHERE well = $1 AND ".to_owned() +
        "tag = $2 AND time >= $3";
    sqlx::query(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).execute(&mut **tx).await.unwrap();
}

pub async fn get_points_f32_lbnsrbs(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF32> {
    // [a,b)
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time < $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f32(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_points_f64_lbnsrbs(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF64> {
    // [a,b)
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time < $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f64(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_points_f32_lbsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF32> {
    // (a,b]
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time > $3 AND time <= $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f32(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_points_f64_lbsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF64> {
    // (a,b]
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time > $3 AND time <= $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f64(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_points_f32_lbsrbs(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF32> {
    // (a,b)
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time > $3 AND time < $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f32(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_points_f64_lbsrbs(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF64> {
    // (a,b)
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time > $3 AND time < $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f64(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_points_f32_lbnsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF32> {
    // [a,b]
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time <= $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f32(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_points_f64_lbnsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF64> {
    // [a,b]
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time <= $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f64(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_points_f64_lbnsrbns_tx(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: &TagF64, time_from: i32, time_to: i32, units: &PresentationUnits)
        -> Vec<PointF64> {
    // [a,b]
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time <= $4 ORDER BY time ASC";
    let mut points = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_all(&mut **tx).await.unwrap();
    if *units == PresentationUnits::US {
        points
    } else {
        for point in points.iter_mut() {
            *point = convert_point_f64(*point, tag, &PresentationUnits::US, &units);
        }
        points
    }
}

pub async fn get_point_before_strict_f32(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time: i32, units: &PresentationUnits) -> Option<PointF32> {
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time < $3 ORDER BY time DESC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time).fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF32 = maybe_point.unwrap();
        let converted_point = convert_point_f32(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_point_before_strict_f64(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, time: i32, units: &PresentationUnits) -> Option<PointF64> {
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time < $3 ORDER BY time DESC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time).fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF64 = maybe_point.unwrap();
        let converted_point = convert_point_f64(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_point_before_nonstrict_f32(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time: i32, units: &PresentationUnits) -> Option<PointF32> {
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time <= $3 ORDER BY time DESC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time).fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF32 = maybe_point.unwrap();
        let converted_point = convert_point_f32(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_point_before_nonstrict_f64(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, time: i32, units: &PresentationUnits) -> Option<PointF64> {
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time <= $3 ORDER BY time DESC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time).fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF64 = maybe_point.unwrap();
        let converted_point = convert_point_f64(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_point_after_nonstrict_f32(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time: i32, units: &PresentationUnits) -> Option<PointF32> {
    let q = "SELECT time, value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 ORDER BY time ASC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time).fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF32 = maybe_point.unwrap();
        let converted_point = convert_point_f32(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_point_after_nonstrict_f64(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, time: i32, units: &PresentationUnits) -> Option<PointF64> {
    let q = "SELECT time, value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 ORDER BY time ASC LIMIT 1";
    let maybe_point = sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time).fetch_optional(&mut **conn).await.unwrap();
    if *units == PresentationUnits::US {
        maybe_point
    } else {
        if maybe_point.is_none() {
            return maybe_point;
        }
        let point: PointF64 = maybe_point.unwrap();
        let converted_point = convert_point_f64(point, tag, &PresentationUnits::US, &units);
        Some(converted_point)
    }
}

pub async fn get_max_value_f32_lbnsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits) -> Option<f32> {
    // [a,b]
    let q = "SELECT MAX(value)::float4 as value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time <= $4";
    let maybe_value =sqlx::query_as::<_, ValueF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_optional(&mut **conn).await.unwrap();
    if maybe_value.is_some() {
        let value_maybe: Option<f32> = maybe_value.unwrap().value;
        if value_maybe.is_some() {
            let value = value_maybe.unwrap();
            if *units == PresentationUnits::US {
                Some(value)
            } else {
                Some(convert_point_f32_value(value, tag, &PresentationUnits::US, &units))
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn get_max_value_f32_lbsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits) -> Option<f32> {
    // (a,b]
    let q = "SELECT MAX(value)::float4 as value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time > $3 AND time <= $4";
    let maybe_value =sqlx::query_as::<_, ValueF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_optional(&mut **conn).await.unwrap();
    if maybe_value.is_some() {
        let value_maybe: Option<f32> = maybe_value.unwrap().value;
        if value_maybe.is_some() {
            let value = value_maybe.unwrap();
            if *units == PresentationUnits::US {
                Some(value)
            } else {
                Some(convert_point_f32_value(value, tag, &PresentationUnits::US, &units))
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn get_avg_value_f32_lbnsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits) -> Option<f32> {
    // [a,b]
    let q = "SELECT AVG(value)::float4 as value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time <= $4";
    let maybe_value =sqlx::query_as::<_, ValueF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_optional(&mut **conn).await.unwrap();
    if maybe_value.is_some() {
        let value_maybe: Option<f32> = maybe_value.unwrap().value;
        if value_maybe.is_some() {
            let value = value_maybe.unwrap();
            if *units == PresentationUnits::US {
                Some(value)
            } else {
                Some(convert_point_f32_value(value, tag, &PresentationUnits::US, &units))
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn get_min_value_f32_lbnsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits) -> Option<f32> {
    // [a,b]
    let q = "SELECT MIN(value)::float4 as value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time <= $4";
    let maybe_value =sqlx::query_as::<_, ValueF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_optional(&mut **conn).await.unwrap();
    if maybe_value.is_some() {
        let value_maybe: Option<f32> = maybe_value.unwrap().value;
        if value_maybe.is_some() {
            let value = value_maybe.unwrap();
            if *units == PresentationUnits::US {
                Some(value)
            } else {
                Some(convert_point_f32_value(value, tag, &PresentationUnits::US, &units))
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn get_min_value_f32_lbsrbns(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF32, time_from: i32, time_to: i32, units: &PresentationUnits) -> Option<f32> {
    // (a,b]
    let q = "SELECT MIN(value)::float4 as value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time > $3 AND time <= $4";
    let maybe_value =sqlx::query_as::<_, ValueF32>(&q).bind(&well.uuid).bind(&tag.id)
        .bind(time_from).bind(time_to).fetch_optional(&mut **conn).await.unwrap();
    if maybe_value.is_some() {
        let value_maybe: Option<f32> = maybe_value.unwrap().value;
        if value_maybe.is_some() {
            let value = value_maybe.unwrap();
            if *units == PresentationUnits::US {
                Some(value)
            } else {
                Some(convert_point_f32_value(value, tag, &PresentationUnits::US, &units))
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn get_interpolated_value_f64(conn: &mut Connection<DB>, well: &Well,
        tag: &TagF64, time: i32, units: &PresentationUnits) -> Option<f64> {
    let point_before_maybe = get_point_before_nonstrict_f64(conn, well,
            tag, time, units).await;
    let point_after_maybe = get_point_after_nonstrict_f64(conn, well,
            tag, time, units).await;
    if point_before_maybe.is_some() {
        let point_before = point_before_maybe.unwrap();
        if point_before.time == time {
            Some(point_before.value)
        } else {
            if point_after_maybe.is_some() {
                let point_after = point_after_maybe.unwrap();
                if point_after.time == time {
                    Some(point_after.value)
                } else {
                    // interpolate
                    let dt: i32 = point_after.time - point_before.time;
                    let k: f64 = (point_after.value - point_before.value)/(dt as f64);
                    let value = point_before.value + k * ((time - point_before.time) as f64);
                    Some(value)
                }
            } else {
                Some(point_before.value)
            }
        }
    } else {
        if point_after_maybe.is_some() {
            let point_after = point_after_maybe.unwrap();
            Some(point_after.value)
        } else {
            None
        }
    }
}

