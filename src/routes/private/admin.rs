use crate::middleware::*;
use rocket::{get, routes, Route};
use rocket_dyn_templates::Template;

#[get("/admin")]
pub async fn get_dashboard(_admin: AdminGuard, app_context: AppContext<'_>) -> Template {
    app_context.render("admin/index")
}

pub fn routes() -> Vec<Route> {
    routes![get_dashboard]
}
