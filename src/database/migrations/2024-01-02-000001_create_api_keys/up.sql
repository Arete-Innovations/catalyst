CREATE TABLE api_keys (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    key_hash VARCHAR NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    last_used_at BIGINT,
    expires_at BIGINT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())),
    updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()))
);

CREATE INDEX api_keys_user_id_idx ON api_keys(user_id);
CREATE UNIQUE INDEX api_keys_key_hash_idx ON api_keys(key_hash);