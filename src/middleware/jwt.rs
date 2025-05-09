use std::env;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header as JWTHeader, Validation};
use rocket::{
    async_trait,
    http::Cookie,
    request::{self, FromRequest, Outcome, Request},
};
use serde::{Deserialize, Serialize};

use crate::{cata_log, meltdown::*, structs::*};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    #[serde(default)]
    pub remember: bool,
}

pub struct JWT(pub Claims);

impl JWT {
    pub fn user_id(&self) -> i32 {
        self.0.sub.parse::<i32>().unwrap()
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for JWT {
    type Error = MeltDown;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookies = request.cookies();
        let token_cookie = match cookies.get("token") {
            Some(cookie) => cookie,
            None => {
                let error = MeltDown::new(MeltType::MissingToken, "No JWT token in cookies");
                cookies.remove(Cookie::new("token", ""));
                cookies.remove(Cookie::new("user_id", ""));
                return Outcome::Error((error.status_code(), error));
            }
        };

        let token = token_cookie.value().to_string();
        let secret = match env::var("JWT_SECRET") {
            Ok(s) => s,
            Err(e) => {
                let error = MeltDown::new(MeltType::ConfigurationError, format!("JWT_SECRET environment variable not set: {}", e));
                cata_log!(Error, error.log_message());
                return Outcome::Error((error.status_code(), error));
            }
        };

        match decode::<Claims>(&token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default()) {
            Ok(token_data) => {
                if token_data.claims.sub.parse::<i32>().is_err() {
                    let error = MeltDown::new(MeltType::InvalidToken, "Invalid user ID format in JWT");
                    cata_log!(Warning, error.log_message());
                    return Outcome::Error((error.status_code(), error));
                }

                if let Some(user_id_cookie) = cookies.get("user_id") {
                    let user_id_from_cookie = user_id_cookie.value();
                    let user_id_from_jwt = &token_data.claims.sub;
                    if user_id_from_cookie != user_id_from_jwt {
                        let error = MeltDown::new(MeltType::InvalidToken, format!("JWT/Cookie User ID mismatch: Cookie='{}', JWT='{}'", user_id_from_cookie, user_id_from_jwt));
                        cata_log!(Warning, error.log_message());
                        cookies.remove(Cookie::new("token", ""));
                        cookies.remove(Cookie::new("user_id", ""));
                        return Outcome::Error((error.status_code(), error));
                    }
                }

                let mut claims = token_data.claims.clone();
                let now_ts = Utc::now().timestamp() as usize;
                let remaining = claims.exp.saturating_sub(now_ts);

                cata_log!(Debug, format!("JWT TTL check for user {}: remaining {} seconds", claims.sub, remaining));

                let (refresh_threshold, extension) = if claims.remember {
                    (Duration::days(1).num_seconds() as usize, Duration::days(7))
                } else {
                    (Duration::hours(1).num_seconds() as usize, Duration::hours(10))
                };
                if remaining < refresh_threshold {
                    let new_exp = (Utc::now() + extension).timestamp() as usize;
                    claims.exp = new_exp;
                    match encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_ref())) {
                        Ok(new_token) => {
                            cata_log!(Debug, format!("Regenerating JWT for user {}: new expiry at {} (in {}s)", claims.sub, new_exp, new_exp.saturating_sub(now_ts)));
                            cookies.add(Cookie::build(Cookie::new("token", new_token)).http_only(true).secure(true).build());
                        }
                        Err(e) => cata_log!(Error, format!("Failed to regenerate JWT: {}", e)),
                    }
                }

                Outcome::Success(JWT(claims))
            }
            Err(e) => {
                let error = MeltDown::from(e);
                cata_log!(Warning, error.log_message());

                cookies.remove(Cookie::new("token", ""));
                cookies.remove(Cookie::new("user_id", ""));
                Outcome::Error((error.status_code(), error))
            }
        }
    }
}

pub async fn jwt_to_user(jwt_token: &str) -> Result<Users, MeltDown> {
    let secret = env::var("JWT_SECRET").map_err(|e| MeltDown::new(MeltType::ConfigurationError, format!("JWT_SECRET not set: {}", e)))?;
    let token_data = decode::<Claims>(jwt_token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default()).map_err(|e| MeltDown::new(MeltType::InvalidToken, format!("Invalid JWT: {}", e)))?;
    let user_id: i32 = token_data.claims.sub.parse().map_err(|e| MeltDown::new(MeltType::InvalidToken, format!("Invalid user ID in JWT: {}", e)))?;
    Users::get_user_by_id(user_id).await
}

pub fn jwt_to_id(jwt: &JWT) -> Result<i32, MeltDown> {
    jwt.0.sub.parse::<i32>().map_err(|e| MeltDown::new(MeltType::InvalidToken, format!("Invalid user ID in JWT: {}", e)))
}
