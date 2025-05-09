use rocket::{get, routes, serde::json::Json, Route};
use serde_json::{json, Value};

#[get("/api/v1/status")]
pub async fn api_status() -> Json<Value> {
    Json(json!({
        "success": true,
        "data": {
            "status": "ok",
            "version": "1.0.0"
        }
    }))
}

pub fn api_v1_routes() -> Vec<Route> {
    routes![api_status]
}
