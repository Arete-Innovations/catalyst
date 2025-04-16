use crate::database::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = api_keys)]
pub struct ApiKeys {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub key_hash: String,
    pub active: bool,
    pub revoked: bool,
    pub last_used_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = api_keys)]
pub struct NewApiKey {
    pub user_id: i32,
    pub name: String,
    pub key_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, Clone)]
#[diesel(table_name = api_request_logs)]
pub struct ApiRequestLogs {
    pub id: i32,
    pub api_key_id: i32,
    pub request_method: String,
    pub request_path: String,
    pub request_ip: String,
    pub request_headers: Option<JsonValue>,
    pub request_content_length: Option<i32>,
    pub request_content_type: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = api_request_logs)]
pub struct NewApiRequestLog {
    pub api_key_id: i32,
    pub request_method: String,
    pub request_path: String,
    pub request_ip: String,
    pub request_headers: Option<JsonValue>,
    pub request_content_length: Option<i32>,
    pub request_content_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = api_response_logs)]
pub struct ApiResponseLogs {
    pub id: i32,
    pub request_log_id: i32,
    pub response_status: i32,
    pub response_time_ms: Option<i32>,
    pub response_content_length: Option<i32>,
    pub response_content_type: Option<String>,
    pub response_headers: Option<JsonValue>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = api_response_logs)]
pub struct NewApiResponseLog {
    pub request_log_id: i32,
    pub response_status: i32,
    pub response_time_ms: Option<i32>,
    pub response_content_length: Option<i32>,
    pub response_content_type: Option<String>,
    pub response_headers: Option<JsonValue>,
}
