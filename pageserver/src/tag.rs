use rocket::serde::{Serialize, Deserialize};
use rocket_db_pools::Connection;

use crate::well::{Well};
use crate::database::DB;

#[derive(PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(FromFormField)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "point_value_size", rename_all = "lowercase")]
pub enum PointValueSize {
    F32,
    F64,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct CustomTag {
    pub well: i16,
    pub id: i16,
    pub value_size: PointValueSize,
    pub units_text: String,
    pub name: String,
    pub description: String,
}

pub async fn get_custom_tags(conn: &mut Connection<DB>, well: &Well) -> Vec<CustomTag> {
    let q = "SELECT * FROM public.custom_tags WHERE well = $1";
    sqlx::query_as::<_, CustomTag>(q).bind(&well.uuid)
        .fetch_all(&mut **conn).await.unwrap()
}

pub struct TagF32 {
    pub id: i16,
}

pub struct TagF64 {
    pub id: i16,
}

