use crate::cata_log;
use crate::models::*;
use crate::services::*;
use crate::structs::*;
use serde::Serialize;

#[derive(Serialize, Debug, Default)]
pub struct ApiKeyContext {
    pub api_key: Option<ApiKeys>,
    pub logs: Option<Vec<ApiKeyLogs>>,
    pub user: Option<Users>,
    pub keys: Option<Vec<ApiKeys>>,
}

impl ApiKeyContext {
    pub async fn build_all(user_id: i32) -> Self {
        cata_log!(Debug, format!("Building API dashboard for user_id: {}", user_id));

        let user = match Users::get_user_by_id(user_id).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let keys = match ApiKeys::get_for_user(user_id) {
            Ok(keys) => Some(keys),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get API keys for user {}: {}", user_id, e));
                None
            }
        };

        let logs = match ApiLogsContext::build_all(user_id).await.logs {
            Some(logs) => Some(logs),
            None => None,
        };

        Self { api_key: None, logs, user, keys }
    }

    pub async fn build_keys(user_id: i32) -> Self {
        cata_log!(Debug, format!("Building API keys list for user_id: {}", user_id));

        let user = match Users::get_user_by_id(user_id).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let keys = match ApiKeys::get_for_user(user_id) {
            Ok(keys) => Some(keys),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get API keys for user {}: {}", user_id, e));
                None
            }
        };

        Self { api_key: None, logs: None, user, keys }
    }

    pub async fn build_key(user_id: i32, key_id: i32) -> Self {
        cata_log!(Debug, format!("Building API key detail for user_id: {} and key_id: {}", user_id, key_id));

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

        let logs = match ApiLogsContext::build_key_logs(user_id, key_id).await.logs {
            Some(logs) => Some(logs),
            None => None,
        };

        Self { api_key, logs, user, keys: None }
    }
}
