use rocket::{
    async_trait,
    http::Status,
    outcome::Outcome::{Error, Forward, Success},
    request::{FromRequest, Outcome, Request},
};

use crate::{cata_log, meltdown::*, middleware::*, structs::*};

fn extract_tenant_from_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if parts.len() >= 2 && parts[1] == "api" {
        let tenant_name = parts[0].to_string();

        if !["api", "auth", "vessel", "admin", "user"].contains(&tenant_name.as_str()) {
            cata_log!(Debug, format!("Extracted tenant name from path: {}", tenant_name));
            return Some(tenant_name);
        }
    }

    cata_log!(Debug, "Could not extract tenant name from path, using default");
    None
}

pub struct AdminGuard;

#[async_trait]
impl<'r> FromRequest<'r> for AdminGuard {
    type Error = MeltDown;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.guard::<JWT>().await {
            Success(jwt) => {
                if jwt.is_admin() {
                    Success(AdminGuard)
                } else {
                    let error = MeltDown::new(MeltType::Forbidden, "Insufficient permissions to access admin area");
                    Error((Status::Forbidden, error))
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
        match auth_header {
            Some(value) if value.starts_with("Bearer ") => {
                let token = value.trim_start_matches("Bearer ").trim();

                if token.is_empty() {
                    let error = MeltDown::new(MeltType::Unauthorized, "Empty API key provided");
                    return Error((Status::Unauthorized, error));
                }

                let request_path = req.uri().path().to_string();
                let tenant_name = extract_tenant_from_path(&request_path).unwrap_or_else(|| "main".to_string());

                match ApiKeys::validate_token(token, &tenant_name).await {
                    Ok(api_key) => Success(ApiKeyGuard(api_key)),
                    Err(_) => {
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
