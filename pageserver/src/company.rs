use rocket::serde::{Serialize, Deserialize};
use rocket_db_pools::Connection;

use crate::user::{User, UserCategory};
use crate::database::DB;

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct Company {
    pub id: String,
    pub name: String,
}

pub async fn get_companies(conn: &mut Connection<DB>, user: &User) -> Vec<Company> {
    if user.category > UserCategory::User {
        let q = "SELECT * FROM public.companies";
        sqlx::query_as::<_, Company>(q).fetch_all(&mut **conn).await.unwrap()
    } else {
        let q = "SELECT * FROM public.companies WHERE id = $1";
        sqlx::query_as::<_, Company>(q).bind(&user.company)
            .fetch_all(&mut **conn).await.unwrap()
    }
}

