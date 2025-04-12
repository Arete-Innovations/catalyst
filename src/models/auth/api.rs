use crate::database::db::establish_connection;
use crate::database::schema::api_key_logs::dsl as api_key_log_dsl;
use crate::database::schema::api_keys::dsl as api_key_dsl;
use crate::meltdown::*;
use crate::structs::*;
use diesel::prelude::*;

impl ApiKeys {
    pub async fn get_api_key_by_token(token: &str) -> Result<ApiKeys, MeltDown> {
        let mut conn = establish_connection();
        let current_timestamp = chrono::Utc::now().timestamp();

        let result = api_key_dsl::api_keys
            .filter(api_key_dsl::key_hash.eq(token))
            .filter(api_key_dsl::active.eq(true))
            .filter(api_key_dsl::revoked.eq(false))
            .filter(api_key_dsl::expires_at.is_null().or(api_key_dsl::expires_at.gt(current_timestamp)))
            .first::<ApiKeys>(&mut conn);

        match result {
            Ok(api_key) => Ok(api_key),
            Err(_) => Err(MeltDown::new(MeltType::InvalidToken, "api_key")),
        }
    }

    pub async fn validate_token(token: &str) -> Result<ApiKeys, MeltDown> {
        let mut conn = establish_connection();
        let current_timestamp = chrono::Utc::now().timestamp();

        let result = api_key_dsl::api_keys
            .filter(api_key_dsl::key_hash.eq(token))
            .filter(api_key_dsl::active.eq(true))
            .filter(api_key_dsl::revoked.eq(false))
            .filter(api_key_dsl::expires_at.is_null().or(api_key_dsl::expires_at.gt(current_timestamp)))
            .first::<ApiKeys>(&mut conn);

        match result {
            Ok(api_key) => {
                diesel::update(api_key_dsl::api_keys.find(api_key.id)).set(api_key_dsl::last_used_at.eq(current_timestamp)).execute(&mut conn).ok();

                Ok(api_key)
            }
            Err(_) => Err(MeltDown::new(MeltType::InvalidToken, "api_key")),
        }
    }
}

impl ApiKeyLogs {
    pub async fn update_api_key_log_status(id: i32, status: i32) -> Result<ApiKeyLogs, MeltDown> {
        let mut conn = establish_connection();

        let result = diesel::update(api_key_log_dsl::api_key_logs.find(id))
            .set(api_key_log_dsl::response_status.eq(status))
            .get_result::<ApiKeyLogs>(&mut conn);

        match result {
            Ok(log) => Ok(log),
            Err(e) => Err(MeltDown::from(e).with_context("operation", "update_api_key_log_status").with_context("id", id.to_string())),
        }
    }
}
