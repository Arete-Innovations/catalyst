use crate::middleware::*;
use rocket::{get, routes, Route};
use rocket_dyn_templates::Template;

#[get("/admin/dashboard")]
pub async fn get_admin_dashboard(app_context: AppContext<'_>) -> Template {
    app_context.render("admin/index")
}

pub fn admin_routes() -> Vec<Route> {
    routes![get_admin_dashboard]
}
