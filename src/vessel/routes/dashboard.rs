use rocket::{get, routes, Route};
use rocket_dyn_templates::Template;
use serde_json::json;

use crate::{cata_log, middleware::*};

#[get("/vessel/dashboard")]
pub async fn get_dashboard(jwt: JWT, app_context: AppContext<'_>) -> Template {
    cata_log!(Info, format!("Vessel accessing dashboard: {}", jwt.get_username()));

    let tenant_name = jwt.get_tenant_name().cloned().unwrap_or_else(|| "unknown".to_string());

    let context = json!({
        "jwt_username": jwt.get_username(),
        "tenant_name": tenant_name
    });

    app_context.render_with("vessel/dashboard", context)
}

pub fn dashboard_routes() -> Vec<Route> {
    routes![get_dashboard]
}
