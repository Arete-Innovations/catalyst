use crate::structs::*;
use jsonwebtoken::{decode, DecodingKey, Validation};
use rocket::async_trait;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub struct JWT(pub Claims);

#[async_trait]
impl<'r> FromRequest<'r> for JWT {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookies = request.cookies();
        let token_cookie = match cookies.get("token") {
            Some(cookie) => cookie,
            None => {
                return Outcome::Error((Status::Unauthorized, ()));
            }
        };

        let token = token_cookie.value().to_string();
        let secret = match env::var("JWT_SECRET") {
            Ok(s) => s,
            Err(_) => {
                eprintln!("FATAL: JWT_SECRET environment variable not set!");
                return Outcome::Error((Status::InternalServerError, ()));
            }
        };

        match decode::<Claims>(&token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default()) {
            Ok(token_data) => {
                if let Some(user_id_cookie) = cookies.get("user_id") {
                    let user_id_from_cookie = user_id_cookie.value();
                    let user_id_from_jwt = &token_data.claims.sub;

                    if user_id_from_cookie != user_id_from_jwt {
                        eprintln!("JWT/Cookie User ID mismatch: Cookie='{}', JWT='{}'", user_id_from_cookie, user_id_from_jwt);
                        return Outcome::Error((Status::Forbidden, ()));
                    }
                } else {
                    eprintln!("No user_id cookie found");
                    return Outcome::Error((Status::Forbidden, ()));
                }

                Outcome::Success(JWT(token_data.claims))
            }
            Err(e) => {
                eprintln!("JWT validation failed: {:?}", e);
                Outcome::Error((Status::Forbidden, ()))
            }
        }
    }
}

pub fn jwt_to_user(jwt_token: &str) -> Result<Users, rocket::http::Status> {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let token_data = decode::<Claims>(jwt_token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default()).map_err(|_| Status::Unauthorized)?;

    let user_id: i32 = token_data.claims.sub.parse().map_err(|_| Status::Unauthorized)?;
    Users::get_user_by_id(user_id).map_err(|_| Status::Unauthorized)
}

pub fn jwt_to_id(jwt: &JWT) -> Result<i32, Status> {
    jwt.0.sub.parse::<i32>().map_err(|_| Status::Unauthorized)
}
