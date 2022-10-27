use sqlx::{Postgres, Transaction, Pool};

use crate::well::Well;

#[derive(Copy, Clone)]
#[derive(sqlx::FromRow)]
pub struct PointF32 {
    pub time: i32,
    pub value: f32,
}

#[derive(Copy, Clone)]
#[derive(sqlx::FromRow)]
pub struct PointF64 {
    pub time: i32,
    pub value: f64,
}

pub async fn get_last_point_f32(tx: &mut Transaction<'_, Postgres>, well: &Well, tag: i16)
        -> Option<PointF32> {
    let q = "SELECT time,value FROM public.points_f32 WHERE ".to_owned() +
        "well = $1 and tag = $2 ORDER BY time DESC LIMIT 1";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag)
        .fetch_optional(&mut *tx).await.unwrap()
}

pub async fn get_last_point_f32_pool(pool: &Pool<Postgres>, well: &Well, tag: i16)
        -> Option<PointF32> {
    let q = "SELECT time,value FROM public.points_f32 WHERE ".to_owned() +
        "well = $1 and tag = $2 ORDER BY time DESC LIMIT 1";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag)
        .fetch_optional(pool).await.unwrap()
}

pub async fn get_last_point_f64(tx: &mut Transaction<'_, Postgres>, well: &Well, tag: i16)
        -> Option<PointF64> {
    let q = "SELECT time,value FROM public.points_f64 WHERE ".to_owned() +
        "well = $1 and tag = $2 ORDER BY time DESC LIMIT 1";
    sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag)
        .fetch_optional(&mut *tx).await.unwrap()
}

pub async fn get_last_point_f64_pool(pool: &Pool<Postgres>, well: &Well, tag: i16)
        -> Option<PointF64> {
    let q = "SELECT time,value FROM public.points_f64 WHERE ".to_owned() +
        "well = $1 and tag = $2 ORDER BY time DESC LIMIT 1";
    sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag)
        .fetch_optional(pool).await.unwrap()
}

pub async fn get_first_point_f32(tx: &mut Transaction<'_, Postgres>, well: &Well, tag: i16)
        -> Option<PointF32> {
    let q = "SELECT time,value FROM public.points_f32 WHERE ".to_owned() +
        "well = $1 and tag = $2 ORDER BY time ASC LIMIT 1";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag)
        .fetch_optional(&mut *tx).await.unwrap()
}

pub async fn get_first_point_f32_pool(pool: &Pool<Postgres>, well: &Well, tag: i16)
        -> Option<PointF32> {
    let q = "SELECT time,value FROM public.points_f32 WHERE ".to_owned() +
        "well = $1 and tag = $2 ORDER BY time ASC LIMIT 1";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag)
        .fetch_optional(pool).await.unwrap()
}

pub async fn get_first_point_f64(tx: &mut Transaction<'_, Postgres>, well: &Well, tag: i16)
        -> Option<PointF64> {
    let q = "SELECT time,value FROM public.points_f64 WHERE ".to_owned() +
        "well = $1 and tag = $2 ORDER BY time ASC LIMIT 1";
    sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag)
        .fetch_optional(&mut *tx).await.unwrap()
}

pub async fn get_first_point_f64_pool(pool: &Pool<Postgres>, well: &Well, tag: i16)
        -> Option<PointF64> {
    let q = "SELECT time,value FROM public.points_f64 WHERE ".to_owned() +
        "well = $1 and tag = $2 ORDER BY time ASC LIMIT 1";
    sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag)
        .fetch_optional(pool).await.unwrap()
}

pub async fn get_recent_point_f32(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: i16, point_number: i16) -> Option<PointF32> {
    let q = "SELECT * FROM (SELECT time, value ".to_owned() +
        "FROM points_f32 WHERE well=$1 AND tag=$2 " +
        "ORDER BY TIME DESC LIMIT $3) points ORDER BY time ASC LIMIT 1";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(tag)
        .bind(point_number)
        .fetch_optional(&mut *tx).await.unwrap()
}

pub async fn get_recent_point_f64(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: i16, point_number: i16) -> Option<PointF64> {
    let q = "SELECT * FROM (SELECT time, value ".to_owned() +
        "FROM points_f64 WHERE well=$1 AND tag=$2 " +
        "ORDER BY TIME DESC LIMIT $3) points ORDER BY time ASC LIMIT 1";
    sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(tag)
        .bind(point_number)
        .fetch_optional(&mut *tx).await.unwrap()
}

pub async fn clear_points_from_nonstrict_f32(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: i16, time: i32) {
    let q = "DELETE FROM public.points_f32 WHERE ".to_owned() +
        "well = $1 AND tag = $2 AND time >= $3";
    sqlx::query(&q).bind(well.uuid).bind(tag).bind(time)
        .execute(&mut **tx).await.unwrap();
}

pub async fn clear_points_from_nonstrict_f64(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: i16, time: i32) {
    let q = "DELETE FROM public.points_f64 WHERE ".to_owned() +
        "well = $1 AND tag = $2 AND time >= $3";
    sqlx::query(&q).bind(well.uuid).bind(tag).bind(time)
        .execute(&mut **tx).await.unwrap();
}

pub async fn insert_point_f32(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: i16, point: PointF32) {
    let q = "INSERT INTO public.points_f32 VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING";
    sqlx::query(&q).bind(well.uuid).bind(tag).bind(point.time)
        .bind(point.value).execute(&mut **tx).await.unwrap();
}

pub async fn insert_point_f64(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: i16, point: PointF64) {
    let q = "INSERT INTO public.points_f64 VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING";
    sqlx::query(&q).bind(well.uuid).bind(tag).bind(point.time)
        .bind(point.value).execute(&mut **tx).await.unwrap();
}

pub async fn insert_points_f32(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: i16, points: &Vec<PointF32>) {
    let len_points: i32 = points.len() as i32;
    if len_points == 0 {
        return;
    }
    let mut q: String = "INSERT INTO public.points_f32 VALUES ".to_owned();
    let mut idx: i32 = 1;
    for point in points.iter() {
        let val_ss: String = format!("('{}',{},{},{})",
            well.uuid, tag, point.time, point.value);
        q += val_ss.as_str();
        if idx != len_points {
            q += ", ";
            idx += 1;
        } else { 
            q += " ON CONFLICT DO NOTHING";
        }
    }
    sqlx::query(&q).execute(&mut *tx).await.unwrap();
}

pub async fn insert_points_f64(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: i16, points: &Vec<PointF64>) {
    let len_points: i32 = points.len() as i32;
    if len_points == 0 {
        return;
    }
    let mut q: String = "INSERT INTO public.points_f64 VALUES ".to_owned();
    let mut idx: i32 = 1;
    for point in points.iter() {
        let val_ss: String = format!("('{}',{},{},{})",
            well.uuid, tag, point.time, point.value);
        q += val_ss.as_str();
        if idx != len_points {
            q += ", ";
            idx += 1;
        } else {
            q += " ON CONFLICT DO NOTHING";
        }
    }
    sqlx::query(&q).execute(&mut *tx).await.unwrap();
}

pub async fn get_points_f32_lbsrbs(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: i16, time_from: i32, time_to: i32) -> Vec<PointF32> {
    // (a,b)
    let q = "SELECT time,value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time > $3 AND time < $4 ORDER BY time ASC";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag)
        .bind(time_from).bind(time_to).fetch_all(&mut *tx).await.unwrap()
}

pub async fn get_points_f32_lbnsrbs(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: i16, time_from: i32, time_to: i32) -> Vec<PointF32> {
    // [a,b)
    let q = "SELECT time,value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time < $4 ORDER BY time ASC";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag)
        .bind(time_from).bind(time_to).fetch_all(&mut *tx).await.unwrap()
}

pub async fn get_points_f32_lbnsrbns(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: i16, time_from: i32, time_to: i32) -> Vec<PointF32> {
    // [a,b]
    let q = "SELECT time,value FROM public.points_f32 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time <= $4 ORDER BY time ASC";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag)
        .bind(time_from).bind(time_to).fetch_all(&mut *tx).await.unwrap()
}

pub async fn get_points_f64_lbnsrbns(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: i16, time_from: i32, time_to: i32) -> Vec<PointF64> {
    // [a,b]
    let q = "SELECT time,value FROM public.points_f64 ".to_owned() +
        "WHERE well = $1 AND tag = $2 AND time >= $3 AND time <= $4 ORDER BY time ASC";
    sqlx::query_as::<_, PointF64>(&q).bind(&well.uuid).bind(&tag)
        .bind(time_from).bind(time_to).fetch_all(&mut *tx).await.unwrap()
}

pub async fn get_point_before_nonstrict_f32(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: i16, time: i32) -> Option<PointF32> {
    let q = "SELECT time,value FROM public.points_f32 ".to_owned() +
        " WHERE well = $1 AND tag = $2 AND time <= $3 ORDER BY time DESC LIMIT 1";
    sqlx::query_as::<_, PointF32>(&q).bind(&well.uuid).bind(&tag)
        .bind(time).fetch_optional(&mut *tx).await.unwrap()
}

pub async fn get_points_f32_lbsrbs_plus_point_before(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: i16, time_from: i32, time_to: i32) -> Vec<PointF32> {
    let point_before_maybe = get_point_before_nonstrict_f32(tx, well, tag, time_from).await;
    if point_before_maybe.is_some() {
        let point_before = point_before_maybe.unwrap();
        get_points_f32_lbnsrbs(tx, well, tag, point_before.time, time_to).await
    } else {
        get_points_f32_lbsrbs(tx, well, tag, time_from, time_to).await
    }
}

