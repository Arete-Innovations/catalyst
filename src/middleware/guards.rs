use crate::cata_log;
use crate::meltdown::*;
use crate::middleware::*;
use crate::structs::*;
use rocket::async_trait;
use rocket::http::Status;
use rocket::outcome::Outcome::{Error, Forward, Success};
use rocket::request::{FromRequest, Outcome, Request};

pub struct AdminGuard;

#[async_trait]
impl<'r> FromRequest<'r> for AdminGuard {
    type Error = MeltDown;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.guard::<JWT>().await {
            Success(jwt) => {
                let user_id = jwt.user_id();
                match Users::is_admin(user_id).await {
                    Ok(is_admin) => {
                        if is_admin {
                            Success(AdminGuard)
                        } else {
                            let error = MeltDown::new(MeltType::Forbidden, "Insufficient permissions to access admin area");
                            Error((Status::Forbidden, error))
                        }
                    }
                    Err(err) => Error((err.status_code(), err)),
                }
            }
            Error((status, error)) => Error((status, error)),
            Forward(status) => Forward(status),
        }
    }
}

pub struct UserGuard;

#[async_trait]
impl<'r> FromRequest<'r> for UserGuard {
    type Error = MeltDown;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.guard::<JWT>().await {
            Success(_) => Success(UserGuard),
            Error((status, error)) => Error((status, error)),
            Forward(status) => Forward(status),
        }
    }
}

pub struct ApiKeyGuard(pub ApiKeys);

#[async_trait]
impl<'r> FromRequest<'r> for ApiKeyGuard {
    type Error = MeltDown;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = req.headers().get_one("Authorization");
        let request_path = req.uri().path().to_string();
        let request_method = req.method().to_string();
        let request_ip = req.client_ip().map(|ip| ip.to_string()).unwrap_or_else(|| "unknown".to_string());

        match auth_header {
            Some(value) if value.starts_with("Bearer ") => {
                let token = value.trim_start_matches("Bearer ").trim();

                if token.is_empty() {
                    let error = MeltDown::new(MeltType::Unauthorized, "Empty API key provided");
                    return Error((Status::Unauthorized, error));
                }

                match ApiKeys::validate_token(token) {
                    Ok(api_key) => {
                        // Log this API request
                        if let Err(e) = ApiKeyLogs::log_request(
                            api_key.id,
                            &request_method,
                            &request_path,
                            &request_ip,
                            200, // We'll assume success; in a real app, we'd update this after the request completes
                        ) {
                            cata_log!(Warning, format!("Failed to log API request: {}", e.log_message()));
                        }

                        Success(ApiKeyGuard(api_key))
                    }
                    Err(_) => {
                        // Create a new MeltDown for the invalid API key
                        let error = MeltDown::new(MeltType::Forbidden, "Invalid API key");
                        Error((Status::Forbidden, error))
                    }
                }
            }
            _ => {
                let error = MeltDown::new(MeltType::Unauthorized, "Missing Authorization header");
                Error((Status::Unauthorized, error))
            }
        }
    }
}

pub struct Referer(pub String);
#[async_trait]
impl<'r> FromRequest<'r> for Referer {
    type Error = MeltDown;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.headers().get_one("Referer") {
            Some(referer) => Outcome::Success(Referer(referer.to_string())),
            None => {
                let error = MeltDown::new(MeltType::MissingField, "Missing Referer header");
                Outcome::Error((Status::BadRequest, error))
            }
        }
    }
}

