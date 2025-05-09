use rocket::{
    async_trait,
    http::Status,
    outcome::Outcome::{Error, Forward, Success},
    request::{FromRequest, Outcome, Request},
};

use crate::{meltdown::*, middleware::*, structs::*};

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
        match auth_header {
            Some(value) if value.starts_with("Bearer ") => {
                let token = value.trim_start_matches("Bearer ").trim();

                if token.is_empty() {
                    let error = MeltDown::new(MeltType::Unauthorized, "Empty API key provided");
                    return Error((Status::Unauthorized, error));
                }

                match ApiKeys::validate_token(token).await {
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
