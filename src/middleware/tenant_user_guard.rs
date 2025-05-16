use rocket::{
    async_trait,
    http::Status,
    outcome::Outcome::{Error, Forward, Success},
    request::{FromRequest, Outcome, Request},
};

use crate::{cata_log, meltdown::*, middleware::*};

pub struct TenantUserGuard {
    pub tenant_name: String,
}

#[async_trait]
impl<'r> FromRequest<'r> for TenantUserGuard {
    type Error = MeltDown;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let jwt = match req.guard::<JWT>().await {
            Success(jwt) => jwt,
            Error((status, error)) => return Error((status, error)),
            Forward(status) => return Forward(status),
        };

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
                "TenantUserGuard checking path: {} with role: {}, auth_system: {:?}, JWT tenant: {:?}, URI tenant: {:?}",
                path,
                jwt.get_role(),
                jwt.get_auth_system(),
                jwt.get_tenant_name(),
                uri_tenant_name
            )
        );

        if jwt.is_vessel_auth() {
            req.local_cache(|| Some(true) as Option<bool>);

            let error = MeltDown::new(MeltType::Forbidden, "Tenant access requires tenant login. Vessel authentication is not sufficient.");
            cata_log!(Warning, format!("Vessel-authenticated user tried to access tenant path: {}", path));
            return Error((Status::Forbidden, error));
        }

        if jwt.is_admin() {
            if let Some(jwt_tenant) = jwt.get_tenant_name() {
                if let Some(uri_tenant) = &uri_tenant_name {
                    if jwt_tenant != uri_tenant {
                        let error = MeltDown::new(MeltType::Forbidden, format!("Admin is authenticated for tenant '{}', not for tenant '{}'", jwt_tenant, uri_tenant));
                        cata_log!(Warning, format!("Admin for tenant '{}' tried to access different tenant: '{}'", jwt_tenant, uri_tenant));
                        return Error((Status::Forbidden, error));
                    }
                }
                cata_log!(Info, format!("Admin access granted to tenant: {}", jwt_tenant));
                return Success(TenantUserGuard { tenant_name: jwt_tenant.clone() });
            } else {
                let error = MeltDown::new(MeltType::Forbidden, "Invalid authentication token - missing tenant information");
                cata_log!(Warning, "Admin token missing tenant information");
                return Error((Status::Forbidden, error));
            }
        }

        let jwt_tenant_name = jwt.get_tenant_name().cloned();

        if let Some(uri_tenant) = uri_tenant_name {
            if !jwt.is_admin() {
                if let Some(jwt_tenant) = &jwt_tenant_name {
                    if *jwt_tenant != uri_tenant {
                        let error = MeltDown::new(MeltType::Forbidden, format!("User has no access to tenant: {}", uri_tenant));
                        cata_log!(Warning, format!("User for tenant '{}' tried to access different tenant: '{}'", jwt_tenant, uri_tenant));
                        return Error((Status::Forbidden, error));
                    }
                } else {
                    let error = MeltDown::new(MeltType::Forbidden, "Tenant-specific access requires tenant association");
                    cata_log!(Warning, format!("User without tenant association tried to access tenant: {}", uri_tenant));
                    return Error((Status::Forbidden, error));
                }
            }

            cata_log!(Info, format!("Access granted to tenant: {}", uri_tenant));
            Success(TenantUserGuard { tenant_name: uri_tenant })
        } else {
            let tenant = jwt_tenant_name.unwrap_or_else(|| "main".to_string());
            cata_log!(Info, format!("No tenant in URI, using tenant: {}", tenant));
            Success(TenantUserGuard { tenant_name: tenant })
        }
    }
}
