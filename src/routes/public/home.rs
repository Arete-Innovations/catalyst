use crate::middleware::*;
use crate::routes::*;
use crate::structs::*;
use rocket::response::Flash;
use rocket::response::Redirect;
use rocket::uri;
use rocket::{get, routes, Route};
use rocket_dyn_templates::Template;

#[get("/")]
pub async fn get_home(app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    if let Some(jwt) = jwt {
        if let Ok(user) = jwt_to_user(&jwt.0.sub) {
            let redirect_uri = if Users::is_admin(user.id) {
                uri!(private::admin::get_dashboard)
            } else {
                uri!(private::user::get_dashboard)
            };
            return Err(Flash::success(Redirect::to(redirect_uri), "Already logged in."));
        }
    }

    Ok(app_context.render("index"))
}

#[get("/oops")]
pub async fn page_not_found(app_context: AppContext<'_>) -> Template {
    app_context.render("oops/index")
}

pub fn routes() -> Vec<Route> {
    routes![get_home, page_not_found]
}
