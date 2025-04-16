-- Create API request logs table
CREATE TABLE api_request_logs (
    id SERIAL PRIMARY KEY,
    api_key_id INTEGER NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    request_method VARCHAR NOT NULL,
    request_path TEXT NOT NULL,
    request_ip VARCHAR NOT NULL,
    request_headers JSONB,
    request_content_length INTEGER,
    request_content_type VARCHAR,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()))
);

-- Create API response logs table
CREATE TABLE api_response_logs (
    id SERIAL PRIMARY KEY,
    request_log_id INTEGER NOT NULL REFERENCES api_request_logs(id) ON DELETE CASCADE,
    response_status INTEGER NOT NULL,
    response_time_ms INTEGER,
    response_content_length INTEGER,
    response_content_type VARCHAR,
    response_headers JSONB,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()))
);

-- Create indexes
CREATE INDEX api_request_logs_api_key_id_idx ON api_request_logs(api_key_id);
CREATE INDEX api_request_logs_created_at_idx ON api_request_logs(created_at);
CREATE INDEX api_response_logs_request_log_id_idx ON api_response_logs(request_log_id);
CREATE INDEX api_response_logs_created_at_idx ON api_response_logs(created_at);
CREATE INDEX api_response_logs_response_status_idx ON api_response_logs(response_status);