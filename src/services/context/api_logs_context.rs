use crate::cata_log;
use crate::structs::*;
use serde::Serialize;

#[derive(Serialize, Debug, Default)]
pub struct ApiLogsContext {
    pub api_key: Option<ApiKeys>,
    pub request_logs: Option<Vec<ApiRequestLogs>>,
    pub response_logs: Option<Vec<ApiResponseLogs>>,
    pub request_log_detail: Option<ApiRequestLogs>,
    pub response_log_detail: Option<ApiResponseLogs>,
    pub user: Option<Users>,
}

impl ApiLogsContext {
    pub async fn build_key_logs(user_id: i32, key_id: i32) -> Self {
        cata_log!(Debug, format!("Building API logs for user_id: {} and key_id: {}", user_id, key_id));

        const DEFAULT_LOG_LIMIT: usize = 100;

        let user = match Users::get_user_by_id(user_id).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let api_key = match ApiKeys::get_by_id(key_id).await {
            Ok(key) => {
                if key.user_id != user_id {
                    cata_log!(Warning, format!("User {} attempted to access key {} belonging to user {}", user_id, key_id, key.user_id));
                    None
                } else {
                    Some(key)
                }
            }
            Err(e) => {
                cata_log!(Warning, format!("Failed to get API key {}: {}", key_id, e));
                None
            }
        };

        let request_logs = if api_key.is_some() {
            match ApiRequestLogs::get_by_api_key_id(key_id).await {
                Ok(logs) => Some(logs.into_iter().take(DEFAULT_LOG_LIMIT).collect()),
                Err(e) => {
                    cata_log!(Warning, format!("Failed to get request logs for API key {}: {}", key_id, e));
                    None
                }
            }
        } else {
            None
        };

        Self { 
            api_key, 
            request_logs, 
            response_logs: None,
            request_log_detail: None, 
            response_log_detail: None,
            user 
        }
    }

    pub async fn build_log_detail(user_id: i32, request_log_id: i32) -> Self {
        cata_log!(Debug, format!("Building log detail for user_id: {} and request_log_id: {}", user_id, request_log_id));

        let user = match Users::get_user_by_id(user_id).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let request_log = match ApiRequestLogs::get_by_id(request_log_id).await {
            Ok(log) => Some(log),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get request log {}: {}", request_log_id, e));
                return Self::default();
            }
        };

        let request_log_detail = request_log.clone();

        let api_key_id = match &request_log {
            Some(l) => l.api_key_id,
            None => return Self::default(),
        };

        let api_key = match ApiKeys::get_by_id(api_key_id).await {
            Ok(key) => {
                if key.user_id != user_id {
                    cata_log!(Warning, format!("User {} attempted to access log for key {} belonging to user {}", user_id, api_key_id, key.user_id));
                    None
                } else {
                    Some(key)
                }
            }
            Err(e) => {
                cata_log!(Warning, format!("Failed to get API key {}: {}", api_key_id, e));
                None
            }
        };

        // Get response log if it exists
        let response_log_detail = match &request_log {
            Some(req_log) => {
                match ApiResponseLogs::get_by_request_log_id(req_log.id).await {
                    Ok(resp_logs) => {
                        if resp_logs.is_empty() {
                            None
                        } else {
                            Some(resp_logs[0].clone())
                        }
                    },
                    Err(e) => {
                        cata_log!(Warning, format!("Failed to get response log for request {}: {}", req_log.id, e));
                        None
                    }
                }
            },
            None => None,
        };

        Self { 
            api_key, 
            request_logs: None, 
            response_logs: None,
            request_log_detail, 
            response_log_detail,
            user 
        }
    }

    pub async fn build_all(user_id: i32) -> Self {
        cata_log!(Debug, format!("Building all API logs for user_id: {}", user_id));

        const DEFAULT_LOG_LIMIT: i64 = 20;

        let user = match Users::get_user_by_id(user_id).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let keys = match ApiKeys::get_by_user_id(user_id).await {
            Ok(keys) => keys,
            Err(e) => {
                cata_log!(Warning, format!("Failed to get API keys for user {}: {}", user_id, e));
                return Self::default();
            }
        };

        if keys.is_empty() {
            return Self::default();
        }

        let key_ids: Vec<i32> = keys.iter().map(|k| k.id).collect();
        let mut request_logs = Vec::new();
        
        for key_id in key_ids.iter() {
            match ApiRequestLogs::get_by_api_key_id(*key_id).await {
                Ok(mut logs) => {
                    request_logs.append(&mut logs);
                }
                Err(e) => {
                    cata_log!(Warning, format!("Failed to get request logs for API key {}: {}", key_id, e));
                }
            }
        }

        Self {
            api_key: None,
            request_logs: Some(request_logs),
            response_logs: None,
            request_log_detail: None,
            response_log_detail: None,
            user,
        }
    }
}