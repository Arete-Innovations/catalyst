use rocket::{get, routes, serde::json::Json, Route};
use serde_json::{json, Value};

use crate::{middleware::*, vessel::structs::Vessel};

#[get("/<tenant>/api/v1/status")]
pub async fn get_api_status(tenant: &str, app_context: AppContext<'_>) -> Json<Value> {
    match Vessel::tenant_exists(tenant).await {
        Ok(exists) => {
            if !exists {
                crate::cata_log!(Warning, format!("API status request for non-existent tenant: {}", tenant));
                return Json(json!({
                    "success": false,
                    "error": {
                        "code": "not_found",
                        "message": "Tenant not found"
                    }
                }));
            }
        }
        Err(e) => {
            crate::cata_log!(Error, format!("Error checking tenant existence: {}", e.log_message()));
            return Json(json!({
                "success": false,
                "error": {
                    "code": "database_error",
                    "message": "Database error"
                }
            }));
        }
    }

    Json(json!({
        "success": true,
        "data": {
            "status": "ok",
            "version": "1.0.0",
            "tenant": tenant
        }
    }))
}

pub fn api_v1_routes() -> Vec<Route> {
    routes![get_api_status]
}
