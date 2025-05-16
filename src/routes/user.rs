use rocket::{get, routes, Route};
use rocket_dyn_templates::Template;

use crate::{meltdown::*, middleware::*, vessel::structs::Vessel};

#[get("/<tenant>/user/dashboard")]
pub async fn get_user_dashboard(tenant: &str, app_context: AppContext<'_>) -> Result<Template, MeltDown> {
    match Vessel::tenant_exists(tenant).await {
        Ok(exists) => {
            if !exists {
                crate::cata_log!(Warning, format!("Attempted to access user dashboard for non-existent tenant: {}", tenant));
                return Err(MeltDown::new(MeltType::NotFound, "Tenant not found"));
            }
        }
        Err(e) => {
            crate::cata_log!(Error, format!("Error checking tenant existence: {}", e.log_message()));
            return Err(MeltDown::new(MeltType::DatabaseError, "Database error"));
        }
    }

    let tenant_data = TenantData::new(tenant, ());
    Ok(app_context.render_with("user/index", tenant_data))
}

pub fn user_routes() -> Vec<Route> {
    routes![get_user_dashboard]
}
