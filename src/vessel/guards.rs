use rocket::{
    async_trait,
    http::Status,
    outcome::Outcome::{Error, Forward, Success},
    request::{FromRequest, Outcome, Request},
};

use crate::{cata_log, meltdown::*, middleware::*};

pub struct VesselHomeGuard;

#[async_trait]
impl<'r> FromRequest<'r> for VesselHomeGuard {
    type Error = MeltDown;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.guard::<JWT>().await {
            Success(jwt) => {
                let path = req.uri().path().as_str();
                cata_log!(Info, format!("VesselHomeGuard checking path: {} with role: {}, auth_system: {:?}", path, jwt.get_role(), jwt.get_auth_system()));

                if !jwt.is_vessel_auth() {
                    let error = MeltDown::new(MeltType::Forbidden, "Vessel area requires vessel authentication. Tenant authentication is not sufficient.");
                    cata_log!(Warning, format!("Non-vessel authenticated user tried to access vessel path: {}", path));
                    return Error((Status::Forbidden, error));
                }

                if jwt.get_role() == "vessel" {
                    Success(VesselHomeGuard)
                } else {
                    let error = MeltDown::new(MeltType::Forbidden, "Insufficient permissions to access vessel area");
                    Error((Status::Forbidden, error))
                }
            }
            Error((status, error)) => Error((status, error)),
            Forward(status) => Forward(status),
        }
    }
}
