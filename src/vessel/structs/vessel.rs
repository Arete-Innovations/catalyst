use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket::form::FromForm;
use serde::{Deserialize, Serialize};

use crate::vessel::database::schema::vessels;

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = vessels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Vessel {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Insertable)]
#[diesel(table_name = vessels)]
pub struct NewVessel {
    pub name: String,
    pub display_name: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub first_name: String,
    pub last_name: String,
}

#[derive(FromForm, Debug)]
pub struct VesselLoginForm {
    pub username: String,
    pub password: String,
    pub remember_me: Option<bool>,
    pub authenticity_token: String,
}

#[derive(FromForm, Debug)]
pub struct VesselRegisterForm {
    pub name: String,
    pub display_name: String,
    pub username: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub first_name: String,
    pub last_name: String,
    pub authenticity_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VesselResponse {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub username: String,
    pub email: String,
    pub active: bool,
}
