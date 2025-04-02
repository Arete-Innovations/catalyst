use crate::middleware::*;
use crate::structs::*;
use rocket::async_trait;
use rocket::http::Status;
use rocket::outcome::Outcome::{Error, Forward, Success};
use rocket::request::{FromRequest, Outcome, Request};

pub struct AdminGuard;

#[async_trait]
impl<'r> FromRequest<'r> for AdminGuard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.guard::<JWT>().await {
            Success(jwt) => match jwt_to_id(&jwt) {
                Ok(user_id) => {
                    if Users::is_admin(user_id) {
                        Success(AdminGuard)
                    } else {
                        Error((Status::Forbidden, ()))
                    }
                }
                Err(status) => Error((status, ())),
            },
            Error((status, _)) => Error((status, ())),
            Forward(status) => Forward(status),
        }
    }
}

pub struct Referer(pub String);
#[async_trait]
impl<'r> FromRequest<'r> for Referer {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.headers().get_one("Referer") {
            Some(referer) => Outcome::Success(Referer(referer.to_string())),
            None => Outcome::Forward(Status::NotFound),
        }
    }
}
