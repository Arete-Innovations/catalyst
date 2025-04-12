use crate::cata_log;
use crate::structs::*;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Request, Response};

#[derive(Copy, Clone)]
struct LogId(i32);

pub struct ApiLogFairing;

#[rocket::async_trait]
impl Fairing for ApiLogFairing {
    fn info(&self) -> Info {
        Info {
            name: "API Request Logger",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        if let Some(auth) = request.headers().get_one("Authorization") {
            if auth.starts_with("Bearer ") {
                let token = auth.trim_start_matches("Bearer ").trim();

                let request_path = request.uri().path().to_string();
                let request_method = request.method().to_string();
                let request_ip = request.client_ip().map(|ip| ip.to_string()).unwrap_or_else(|| "unknown".to_string());

                if let Ok(api_key) = ApiKeys::get_api_key_by_token(token).await {
                    let new_api_log = NewApiKeyLogs {
                        api_key_id: api_key.id,
                        request_method,
                        request_path,
                        request_ip,
                        response_status: 0,
                    };
                    match ApiKeyLogs::create(new_api_log).await {
                        Ok(log) => {
                            request.local_cache(|| LogId(log.id));
                            cata_log!(Debug, format!("API request pre-logged with temporary ID {}", log.id));
                        }
                        Err(e) => {
                            cata_log!(Warning, format!("Failed to log API request: {}", e));
                        }
                    }
                } else {
                    cata_log!(Info, format!("No valid API key found for token prefix"));
                }
            }
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if let Some(log_id_wrapper) = request.local_cache(|| Option::<LogId>::None) {
            let id = log_id_wrapper.0;
            let status = response.status().code as i32;

            if let Err(e) = ApiKeyLogs::update_api_key_log_status(id, status).await {
                cata_log!(Warning, format!("Failed to update API log status for ID {}: {}", id, e));
            } else {
                cata_log!(Debug, format!("Updated API log {} with status {}", id, status));
            }
        }
    }
}
