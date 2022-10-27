use sqlx::{Postgres, Transaction};
use crate::well::Well;

#[derive(PartialEq)]
#[derive(sqlx::Type)]
#[sqlx(type_name = "point_value_size", rename_all = "lowercase")]
pub enum PointValueSize {
    F32,
    F64,
}

#[derive(sqlx::FromRow)]
pub struct Tag {
    pub id: i16,
    pub value_size: PointValueSize,
}

pub async fn get_custom_tags(tx: &mut Transaction<'_, Postgres>, well: &Well)
        -> Vec<Tag> {
    let q = "SELECT id, value_size FROM public.custom_tags WHERE well = $1";
    sqlx::query_as::<_, Tag>(q).bind(&well.uuid)
        .bind(well.uuid).fetch_all(&mut *tx).await.unwrap()    
}
