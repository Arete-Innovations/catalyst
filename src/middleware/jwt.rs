use std::env;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header as JWTHeader, Validation};
use rocket::{
    async_trait,
    http::Cookie,
    request::{self, FromRequest, Outcome, Request},
};
use serde::{Deserialize, Serialize};

use crate::{bootstrap, cata_log, meltdown::*, services::default::*, structs::*};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TokenType {
    #[serde(rename = "access")]
    Access,
    #[serde(rename = "refresh")]
    Refresh,
}

impl Default for TokenType {
    fn default() -> Self {
        TokenType::Access
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub jti: String,
    pub iat: usize,
    pub nbf: usize,
    #[serde(default = "default_version")]
    pub ver: u32,
    #[serde(default)]
    pub remember: bool,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub token_type: TokenType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_jti: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_info: Option<String>,
}

fn default_version() -> u32 {
    1
}

pub struct JWT(pub Claims);

impl JWT {
    pub fn user_id(&self) -> i32 {
        self.0.sub.parse::<i32>().unwrap()
    }

    pub fn is_admin(&self) -> bool {
        self.0.role == "admin"
    }

    pub fn get_role(&self) -> &str {
        &self.0.role
    }

    pub fn get_username(&self) -> &str {
        &self.0.username
    }

    pub fn get_jti(&self) -> &str {
        &self.0.jti
    }

    pub fn get_version(&self) -> u32 {
        self.0.ver
    }

    pub fn get_expiration(&self) -> usize {
        self.0.exp
    }

    pub fn get_issued_at(&self) -> usize {
        self.0.iat
    }

    pub fn is_access_token(&self) -> bool {
        self.0.token_type == TokenType::Access
    }

    pub fn is_refresh_token(&self) -> bool {
        self.0.token_type == TokenType::Refresh
    }

    pub fn get_token_type(&self) -> &TokenType {
        &self.0.token_type
    }

    pub fn get_refresh_jti(&self) -> Option<&String> {
        self.0.refresh_jti.as_ref()
    }

    pub fn get_device_info(&self) -> Option<&String> {
        self.0.device_info.as_ref()
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for JWT {
    type Error = MeltDown;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookies = request.cookies();
        let token_cookie = match cookies.get("access_token") {
            Some(cookie) => cookie,
            None => {
                let error = MeltDown::new(MeltType::MissingToken, "No access token in cookies");
                cookies.remove(Cookie::new("access_token", ""));
                cookies.remove(Cookie::new("user_id", ""));
                return Outcome::Error((error.status_code(), error));
            }
        };

        let token = token_cookie.value().to_string();

        match validate_token(&token) {
            Ok(claims) => {
                if claims.token_type == TokenType::Refresh {
                    let error = MeltDown::new(MeltType::InvalidToken, "Refresh token cannot be used for authentication");
                    cata_log!(Warning, error.log_message());
                    return Outcome::Error((error.status_code(), error));
                }

                if claims.sub.parse::<i32>().is_err() {
                    let error = MeltDown::new(MeltType::InvalidToken, "Invalid user ID format in JWT");
                    cata_log!(Warning, error.log_message());
                    return Outcome::Error((error.status_code(), error));
                }

                if let Some(user_id_cookie) = cookies.get("user_id") {
                    let user_id_from_cookie = user_id_cookie.value();
                    let user_id_from_jwt = &claims.sub;
                    if user_id_from_cookie != user_id_from_jwt {
                        let error = MeltDown::new(MeltType::InvalidToken, format!("JWT/Cookie User ID mismatch: Cookie='{}', JWT='{}'", user_id_from_cookie, user_id_from_jwt));
                        cata_log!(Warning, error.log_message());
                        cookies.remove(Cookie::new("access_token", ""));
                        cookies.remove(Cookie::new("user_id", ""));
                        return Outcome::Error((error.status_code(), error));
                    }
                }

                Outcome::Success(JWT(claims))
            }
            Err(e) => {
                let error = MeltDown::from(e);
                cata_log!(Warning, error.log_message());

                cookies.remove(Cookie::new("access_token", ""));
                cookies.remove(Cookie::new("user_id", ""));
                Outcome::Error((error.status_code(), error))
            }
        }
    }
}

pub async fn jwt_to_user(jwt_token: &str) -> Result<Users, MeltDown> {
    let claims = validate_token(jwt_token)?;

    let user_id: i32 = claims.sub.parse().map_err(|e| MeltDown::new(MeltType::InvalidToken, format!("Invalid user ID in JWT: {}", e)))?;

    Users::get_user_by_id(user_id).await
}

pub fn jwt_to_id(jwt: &JWT) -> Result<i32, MeltDown> {
    jwt.0.sub.parse::<i32>().map_err(|e| MeltDown::new(MeltType::InvalidToken, format!("Invalid user ID in JWT: {}", e)))
}
