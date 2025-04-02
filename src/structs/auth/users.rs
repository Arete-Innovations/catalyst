use crate::database::schema::users;
use diesel::prelude::*;
use diesel::Queryable;
use rocket::FromForm;
use serde::Deserialize;
use serde::Serialize;

#[derive(Queryable, QueryableByName, Debug, Identifiable, Serialize, Deserialize, Default)]
#[diesel(table_name = users)]
pub struct Users {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub password_hash: String,
    pub role: String,
    pub active: bool,
    pub should_change_password: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub role: String,
}

#[derive(FromForm)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub authenticity_token: String,
}

#[derive(FromForm, Deserialize, Serialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub password: String,
    pub confirm_password: String,
    pub authenticity_token: String,
}

#[derive(FromForm, Deserialize, Serialize)]
pub struct UpdatePassword<'a> {
    pub password: &'a str,
    pub confirm_password: &'a str,
}
