use rocket::{get, routes, Route};

use crate::middleware::*;

#[get("/<tenant>/user/partials/post_list")]
pub async fn get_users_table(tenant: &str, app_context: AppContext<'_>) -> String {
    format!("Users Table Partial for tenant: {}", tenant)
}

pub fn user_partial_routes() -> Vec<Route> {
    routes![get_users_table]
}
