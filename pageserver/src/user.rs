use rocket::serde::{Serialize, Deserialize};
use rocket::FromFormField;
use rocket_db_pools::Connection;

use crate::database::DB;
use crate::company::Company;

#[derive(PartialEq, PartialOrd)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "user_category", rename_all = "lowercase")]
#[derive(FromFormField)]
pub enum UserCategory {
    User,
    Staff,
    Admin,
}

#[derive(PartialEq, PartialOrd)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "web_access_enum", rename_all = "lowercase")]
#[derive(FromFormField)]
pub enum WebAccess {
    Blocked,
    Readonly,
    Full,
}

#[derive(PartialEq, PartialOrd)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "api_access_enum", rename_all = "lowercase")]
#[derive(FromFormField)]
pub enum ApiAccess {
    Blocked,
    Readonly,
    Full,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[derive(sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub uuid: i16,
    pub category: UserCategory,
    pub company: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub last_active: i32,
    pub web_access: WebAccess,
    pub api_access: ApiAccess,
}

pub async fn get_users(conn: &mut Connection<DB>, company: &Company) -> Vec<User> {
    let q = "SELECT id,uuid,category,company,first_name,last_name,email,".to_owned() +
        "last_active,web_access,api_access FROM public.users WHERE company = $1";
    sqlx::query_as::<_, User>(&q).bind(&company.id)
        .fetch_all(&mut **conn).await.unwrap()
}

pub async fn get_user(conn: &mut Connection<DB>, userid: &str) -> Option<User> {
    let q = "SELECT id,uuid,category,company,first_name,".to_owned() +
        "last_name,email,last_active,web_access,api_access " +
        "FROM public.users WHERE id = $1";
    sqlx::query_as::<_, User>(&q).bind(userid)
        .fetch_optional(&mut **conn).await.unwrap()
}

pub async fn get_user_uuid(conn: &mut Connection<DB>, uuid: i16) -> Option<User> {
    let q = "SELECT id,uuid,category,company,first_name,".to_owned() +
        "last_name,email,last_active,web_access,api_access " +
        "FROM public.users WHERE uuid = $1";
    sqlx::query_as::<_, User>(&q).bind(uuid)
        .fetch_optional(&mut **conn).await.unwrap()
}

