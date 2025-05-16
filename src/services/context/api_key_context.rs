use serde::Serialize;

use crate::{cata_log, services::context::api_logs_context::ApiLogsContext, structs::*};

#[derive(Serialize, Debug)]
pub struct ApiKeyContext {
    pub api_key: Option<ApiKeys>,
    pub request_logs: Option<Vec<ApiRequestLogs>>,
    pub response_logs: Option<Vec<ApiResponseLogs>>,
    pub user: Option<Users>,
    pub keys: Option<Vec<ApiKeys>>,
}

impl ApiKeyContext {
    pub async fn build_all(user_id: i32, tenant_name: &str) -> Self {
        cata_log!(Debug, format!("Building API dashboard for user_id: {} with tenant: {}", user_id, tenant_name));

        let user = match Users::get_user_by_id(user_id, tenant_name).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let keys = match ApiKeys::get_by_user_id(user_id, tenant_name).await {
            Ok(keys) => Some(keys),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get API keys for user {}: {}", user_id, e));
                None
            }
        };

        let logs_context = ApiLogsContext::build_all(user_id, tenant_name).await;
        let request_logs = logs_context.request_logs;

        Self {
            api_key: None,
            request_logs,
            response_logs: None,
            user,
            keys,
        }
    }

    pub async fn build_keys(user_id: i32, tenant_name: &str) -> Self {
        cata_log!(Debug, format!("Building API keys list for user_id: {} with tenant: {}", user_id, tenant_name));

        let user = match Users::get_user_by_id(user_id, tenant_name).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let keys = match ApiKeys::get_by_user_id(user_id, tenant_name).await {
            Ok(keys) => Some(keys),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get API keys for user {}: {}", user_id, e));
                None
            }
        };

        Self {
            api_key: None,
            request_logs: None,
            response_logs: None,
            user,
            keys,
        }
    }

    pub async fn build_key(user_id: i32, key_id: i32, tenant_name: &str) -> Self {
        cata_log!(Debug, format!("Building API key detail for user_id: {} and key_id: {} with tenant: {}", user_id, key_id, tenant_name));

        let user = match Users::get_user_by_id(user_id, tenant_name).await {
            Ok(user) => Some(user),
            Err(e) => {
                cata_log!(Warning, format!("Failed to get user {}: {}", user_id, e));
                None
            }
        };

        let api_key = match ApiKeys::get_by_id(key_id, tenant_name).await {
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

        let logs_context = ApiLogsContext::build_key_logs(user_id, key_id, tenant_name).await;
        let request_logs = logs_context.request_logs;

        Self {
            api_key,
            request_logs,
            response_logs: None,
            user,
            keys: None,
        }
    }
}
