use rocket_db_pools::{Database, deadpool_redis};
use rocket_db_pools::sqlx::PgPool;

#[derive(Database)]
#[database("geotool")]
pub struct DB(PgPool);

#[derive(Database)]
#[database("redis")]
pub struct MEMDB(deadpool_redis::Pool);

