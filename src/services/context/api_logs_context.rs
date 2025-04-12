use crate::cata_log;
use crate::models::*;
use crate::structs::*;
use serde::Serialize;

#[derive(Serialize, Debug, Default)]
pub struct ApiLogsContext {
    pub api_key: Option<ApiKeys>,
    pub logs: Option<Vec<ApiKeyLogs>>,
    pub log_detail: Option<ApiKeyLogs>,
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

        let api_key = match get_api_key_by_id(key_id) {
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

        let logs = if api_key.is_some() {
            match ApiKeyLogs::get_for_key(key_id) {
                Ok(logs) => Some(logs.into_iter().take(DEFAULT_LOG_LIMIT).collect()),
                Err(e) => {
                    cata_log!(Warning, format!("Failed to get logs for API key {}: {}", key_id, e));
                    None
                }
            }
        } else {
            None
        };

        Self { api_key, logs, log_detail: None, user }
    }

    pub async fn build_log_detail(user_id: i32, log_id: i32) -> Self {
        cata_log!(Debug, format!("Building log detail for user_id: {} and log_id: {}", user_id, log_id));

        let user = match Users::get_user_by_id(user_id).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let log = match get_api_key_log_by_id(log_id) {
            Ok(log) => Some(log),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get log {}: {}", log_id, e));
                return Self::default();
            }
        };

        let log_detail = log.clone();

        let api_key_id = match &log {
            Some(l) => l.api_key_id,
            None => return Self::default(),
        };

        let api_key = match get_api_key_by_id(api_key_id) {
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

        Self { api_key, logs: None, log_detail, user }
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

        let keys = match ApiKeys::get_for_user(user_id) {
            Ok(keys) => keys,
            Err(e) => {
                cata_log!(Warning, format!("Failed to get API keys for user {}: {}", user_id, e));
                return Self {
                    api_key: None,
                    logs: None,
                    log_detail: None,
                    user,
                };
            }
        };

        if keys.is_empty() {
            return Self {
                api_key: None,
                logs: None,
                log_detail: None,
                user,
            };
        }

        let key_ids: Vec<i32> = keys.iter().map(|k| k.id).collect();

        let logs = match ApiKeyLogs::get_for_multiple_keys(&key_ids, DEFAULT_LOG_LIMIT) {
            Ok(logs) => Some(logs),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get logs for user {}'s keys: {}", user_id, e));
                None
            }
        };

        Self {
            api_key: None,
            logs,
            log_detail: None,
            user,
        }
    }
}
