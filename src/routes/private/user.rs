use crate::middleware::*;
use crate::structs::*;
use rocket::{get, routes, Route};
use rocket_dyn_templates::Template;

#[get("/user")]
pub async fn get_dashboard(app_context: AppContext<'_>, jwt: JWT) -> Template {
    let user_id = jwt_to_id(&jwt).unwrap();
    let user = Users::get_user_by_id(user_id).unwrap();
    app_context.render_with("user/index", user)
}

pub fn routes() -> Vec<Route> {
    routes![get_dashboard]
}
