diesel::table! {
    api_keys (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Text,
        key_hash -> Varchar,
        active -> Bool,
        revoked -> Bool,
        last_used_at -> Nullable<Int8>,
        expires_at -> Nullable<Int8>,
        created_at -> Int8,
        updated_at -> Int8,
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
        #[max_length = 100]
        title -> Varchar,
        content -> Text,
        public -> Bool,
        created_at -> Int8,
        updated_at -> Int8,
    }
}

diesel::table! {
    spatial_ref_sys (srid) {
        srid -> Int4,
        #[max_length = 256]
        auth_name -> Nullable<Varchar>,
        auth_srid -> Nullable<Int4>,
        #[max_length = 2048]
        srtext -> Nullable<Varchar>,
        #[max_length = 2048]
        proj4text -> Nullable<Varchar>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Text,
        email -> Nullable<Text>,
        first_name -> Text,
        last_name -> Text,
        password_hash -> Varchar,
        role -> Text,
        active -> Bool,
        should_change_password -> Bool,
        created_at -> Int8,
        updated_at -> Int8,
    }
}

diesel::joinable!(api_keys -> users (user_id));
diesel::joinable!(api_request_logs -> api_keys (api_key_id));
diesel::joinable!(api_response_logs -> api_request_logs (request_log_id));
diesel::joinable!(posts -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(api_keys, api_request_logs, api_response_logs, cronjobs, posts, spatial_ref_sys, users,);
