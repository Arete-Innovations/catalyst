use rocket::{
    async_trait,
    http::Status,
    outcome::Outcome::{Error, Forward, Success},
    request::{FromRequest, Outcome, Request},
};

use crate::{cata_log, meltdown::*, middleware::*};

pub struct TenantAdminGuard {
    pub tenant_name: String,
}

#[async_trait]
impl<'r> FromRequest<'r> for TenantAdminGuard {
    type Error = MeltDown;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.guard::<JWT>().await {
            Success(jwt) => {
                let path = req.uri().path().as_str();
                let uri_tenant_name = req.uri().path().split('/').nth(1).and_then(|segment| {
                    if segment.is_empty() || segment.contains("@") || segment.contains(".") {
                        None
                    } else {
                        Some(segment.to_string())
                    }
                });

                cata_log!(
                    Info,
                    format!(
                        "TenantAdminGuard checking path: {} with role: {}, auth_system: {:?}, JWT tenant: {:?}, URI tenant: {:?}",
                        path,
                        jwt.get_role(),
                        jwt.get_auth_system(),
                        jwt.get_tenant_name(),
                        uri_tenant_name
                    )
                );

                if jwt.is_vessel_auth() {
                    let error = MeltDown::new(MeltType::Forbidden, "Admin access requires tenant admin login. Vessel authentication is not sufficient.");
                    cata_log!(Warning, format!("Vessel-authenticated user tried to access admin path: {}", path));
                    return Error((Status::Forbidden, error));
                }

                if !jwt.is_admin() {
                    let error = MeltDown::new(MeltType::Forbidden, "Insufficient permissions to access admin area");
                    cata_log!(Warning, format!("Non-admin user tried to access admin path: {}", path));
                    return Error((Status::Forbidden, error));
                }

                let tenant_name = uri_tenant_name.unwrap_or_else(|| "main".to_string());

                if let Some(jwt_tenant) = jwt.get_tenant_name() {
                    if *jwt_tenant != tenant_name {
                        let error = MeltDown::new(MeltType::Forbidden, format!("Authentication is for tenant '{}', not for tenant '{}'", jwt_tenant, tenant_name));
                        cata_log!(Warning, format!("Admin for tenant '{}' tried to access different tenant: '{}'", jwt_tenant, tenant_name));
                        return Error((Status::Forbidden, error));
                    }
                } else {
                    let error = MeltDown::new(MeltType::Forbidden, "Invalid authentication token - missing tenant information");
                    cata_log!(Warning, "Admin token missing tenant information");
                    return Error((Status::Forbidden, error));
                }

                cata_log!(Info, format!("Admin access granted to tenant: {}", tenant_name));
                Success(TenantAdminGuard { tenant_name })
            }
            Error((status, error)) => Error((status, error)),
            Forward(status) => Forward(status),
        }
    }
}
