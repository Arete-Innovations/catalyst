CREATE TABLE api_key_logs (
    id SERIAL PRIMARY KEY,
    api_key_id INTEGER NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    request_method VARCHAR NOT NULL,
    request_path TEXT NOT NULL,
    request_ip VARCHAR NOT NULL,
    response_status INTEGER NOT NULL,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()))
);

CREATE INDEX api_key_logs_api_key_id_idx ON api_key_logs(api_key_id);
CREATE INDEX api_key_logs_created_at_idx ON api_key_logs(created_at);