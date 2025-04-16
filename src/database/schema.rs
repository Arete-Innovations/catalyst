// @generated automatically by Diesel CLI.

diesel::table! {
    api_keys (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Varchar,
        key_hash -> Varchar,
        active -> Bool,
        revoked -> Bool,
        created_at -> Int8,
        updated_at -> Int8,
        expires_at -> Nullable<Int8>,
        last_used_at -> Nullable<Int8>,
    }
}

diesel::table! {
    api_request_logs (id) {
        id -> Int4,
        api_key_id -> Int4,
        request_method -> Varchar,
        request_path -> Text,
        request_ip -> Varchar,
        request_headers -> Nullable<Jsonb>,
        request_content_length -> Nullable<Int4>,
        request_content_type -> Nullable<Varchar>,
        created_at -> Int8,
    }
}

diesel::table! {
    api_response_logs (id) {
        id -> Int4,
        request_log_id -> Int4,
        response_status -> Int4,
        response_time_ms -> Nullable<Int4>,
        response_content_length -> Nullable<Int4>,
        response_content_type -> Nullable<Varchar>,
        response_headers -> Nullable<Jsonb>,
        created_at -> Int8,
    }
}

diesel::table! {
    posts (id) {
        id -> Int4,
        user_id -> Int4,
        title -> Varchar,
        content -> Text,
        published -> Bool,
        created_at -> Int8,
        updated_at -> Int8,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Varchar,
        role -> Varchar,
        active -> Bool,
        created_at -> Int8,
        updated_at -> Int8,
    }
}

diesel::joinable!(api_request_logs -> api_keys (api_key_id));
diesel::joinable!(api_response_logs -> api_request_logs (request_log_id));
diesel::joinable!(posts -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_keys,
    api_request_logs,
    api_response_logs,
    posts,
    users,
);