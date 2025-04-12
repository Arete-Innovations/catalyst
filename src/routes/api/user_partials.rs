use rocket::{get, routes, Route};

#[get("/user/partials/post_list")]
pub async fn users_table() -> &'static str {
    "Users Table Partial"
}

pub fn user_partial_routes() -> Vec<Route> {
    routes![users_table]
}
