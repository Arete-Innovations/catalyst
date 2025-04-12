use crate::meltdown::*;
use crate::middleware::*;
use rocket::{get, routes, Route};
use rocket_dyn_templates::Template;

#[get("/user/dashboard")]
pub async fn get_user_dashboard(app_context: AppContext<'_>) -> Result<Template, MeltDown> {
    Ok(app_context.render("user/index"))
}

pub fn user_routes() -> Vec<Route> {
    routes![get_user_dashboard]
}
