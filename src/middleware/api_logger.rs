use crate::cata_log;
use crate::structs::*;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Request, Response};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone)]
struct RequestInfo {
    id: i32,
    start_time: Instant,
    content_type: Option<String>,
    content_length: Option<i32>,
}

pub struct ApiLogFairing;

#[rocket::async_trait]
impl Fairing for ApiLogFairing {
    fn info(&self) -> Info {
        Info {
            name: "API Request/Response Logger",
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

                let mut headers = HashMap::new();
                for header in request.headers().iter() {
                    if header.name() != "Authorization" {
                        headers.insert(header.name().to_string(), header.value().to_string());
                    }
                }

                let headers_json = serde_json::to_value(headers).unwrap_or(JsonValue::Null);

                let content_type = request.headers().get_one("Content-Type").map(|s| s.to_string());
                let content_length = request.headers().get_one("Content-Length").and_then(|cl| cl.parse::<i32>().ok());

                if let Ok(api_key) = ApiKeys::get_api_key_by_token(token).await {
                    let new_request_log = NewApiRequestLog {
                        api_key_id: api_key.id,
                        request_method,
                        request_path,
                        request_ip,
                        request_headers: Some(headers_json),
                        request_content_type: content_type.clone(),
                        request_content_length: content_length,
                    };

                    match ApiRequestLogs::create(new_request_log).await {
                        Ok(log) => {
                            let request_info = RequestInfo {
                                id: log.id,
                                start_time: Instant::now(),
                                content_type,
                                content_length,
                            };

                            request.local_cache(|| request_info);
                            cata_log!(Debug, format!("API request logged with ID {}", log.id));
                        }
                        Err(e) => {
                            cata_log!(Warning, format!("Failed to log API request: {}", e));
                        }
                    }
                } else {
                    cata_log!(Info, format!("No valid API key found for token"));
                }
            }
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if let Some(request_info) = request.local_cache(|| Option::<RequestInfo>::None) {
            let id = request_info.id;
            let status = response.status().code as i32;
            let elapsed_time = request_info.start_time.elapsed();
            let response_time_ms = elapsed_time.as_millis() as i32;

            let mut headers = HashMap::new();
            for header in response.headers().iter() {
                headers.insert(header.name().to_string(), header.value().to_string());
            }

            let headers_json = serde_json::to_value(headers).unwrap_or(JsonValue::Null);

            let content_type = response.content_type().map(|ct| ct.to_string());
            let content_length = response.headers().get_one("Content-Length").and_then(|v| v.parse::<i32>().ok());

            let new_response_log = NewApiResponseLog {
                request_log_id: id,
                response_status: status,
                response_time_ms: Some(response_time_ms),
                response_content_type: content_type,
                response_content_length: content_length.map(|len| len as i32),
                response_headers: Some(headers_json),
            };

            match ApiResponseLogs::create(new_response_log).await {
                Ok(log) => {
                    cata_log!(Debug, format!("API response logged with ID {} for request {}", log.id, id));
                }
                Err(e) => {
                    cata_log!(Warning, format!("Failed to log API response for request {}: {}", id, e));
                }
            }
        }
    }
}
