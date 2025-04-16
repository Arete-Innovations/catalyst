use crate::database::db::establish_connection;
use crate::database::schema::api_keys::dsl as api_key_dsl;
use crate::database::schema::api_request_logs::dsl as api_request_log_dsl;
use crate::database::schema::api_response_logs::dsl as api_response_log_dsl;
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

    pub async fn get_by_id(id: i32) -> Result<ApiKeys, MeltDown> {
        let mut conn = establish_connection();

        let result = api_key_dsl::api_keys.find(id).first::<ApiKeys>(&mut conn);

        match result {
            Ok(api_key) => Ok(api_key),
            Err(e) => Err(MeltDown::from(e).with_context("operation", "get_api_key_by_id").with_context("id", id.to_string())),
        }
    }

    pub async fn get_by_user_id(user_id: i32) -> Result<Vec<ApiKeys>, MeltDown> {
        let mut conn = establish_connection();

        let result = api_key_dsl::api_keys.filter(api_key_dsl::user_id.eq(user_id)).load::<ApiKeys>(&mut conn);

        match result {
            Ok(api_keys) => Ok(api_keys),
            Err(e) => Err(MeltDown::from(e).with_context("operation", "get_api_keys_by_user_id").with_context("user_id", user_id.to_string())),
        }
    }
}

impl ApiRequestLogs {
    pub async fn create(new_log: NewApiRequestLog) -> Result<ApiRequestLogs, MeltDown> {
        let mut conn = establish_connection();

        let result = diesel::insert_into(api_request_log_dsl::api_request_logs).values(&new_log).get_result(&mut conn);

        match result {
            Ok(log) => Ok(log),
            Err(e) => Err(MeltDown::from(e).with_context("operation", "create_api_request_log")),
        }
    }

    pub async fn get_by_id(id: i32) -> Result<ApiRequestLogs, MeltDown> {
        let mut conn = establish_connection();

        let result = api_request_log_dsl::api_request_logs.find(id).first::<ApiRequestLogs>(&mut conn);

        match result {
            Ok(log) => Ok(log),
            Err(e) => Err(MeltDown::from(e).with_context("operation", "get_api_request_log_by_id").with_context("id", id.to_string())),
        }
    }

    pub async fn get_by_api_key_id(api_key_id: i32) -> Result<Vec<ApiRequestLogs>, MeltDown> {
        let mut conn = establish_connection();

        let result = api_request_log_dsl::api_request_logs
            .filter(api_request_log_dsl::api_key_id.eq(api_key_id))
            .order(api_request_log_dsl::created_at.desc())
            .load::<ApiRequestLogs>(&mut conn);

        match result {
            Ok(logs) => Ok(logs),
            Err(e) => Err(MeltDown::from(e).with_context("operation", "get_api_request_logs_by_api_key_id").with_context("api_key_id", api_key_id.to_string())),
        }
    }
}

impl ApiResponseLogs {
    pub async fn create(new_log: NewApiResponseLog) -> Result<ApiResponseLogs, MeltDown> {
        let mut conn = establish_connection();

        let result = diesel::insert_into(api_response_log_dsl::api_response_logs).values(&new_log).get_result(&mut conn);

        match result {
            Ok(log) => Ok(log),
            Err(e) => Err(MeltDown::from(e).with_context("operation", "create_api_response_log")),
        }
    }

    pub async fn get_by_id(id: i32) -> Result<ApiResponseLogs, MeltDown> {
        let mut conn = establish_connection();

        let result = api_response_log_dsl::api_response_logs.find(id).first::<ApiResponseLogs>(&mut conn);

        match result {
            Ok(log) => Ok(log),
            Err(e) => Err(MeltDown::from(e).with_context("operation", "get_api_response_log_by_id").with_context("id", id.to_string())),
        }
    }

    pub async fn get_by_request_log_id(request_log_id: i32) -> Result<Vec<ApiResponseLogs>, MeltDown> {
        let mut conn = establish_connection();

        let result = api_response_log_dsl::api_response_logs
            .filter(api_response_log_dsl::request_log_id.eq(request_log_id))
            .order(api_response_log_dsl::created_at.desc())
            .load::<ApiResponseLogs>(&mut conn);

        match result {
            Ok(logs) => Ok(logs),
            Err(e) => Err(MeltDown::from(e)
                .with_context("operation", "get_api_response_logs_by_request_log_id")
                .with_context("request_log_id", request_log_id.to_string())),
        }
    }
}
