INSERT INTO api_keys (user_id, name, key_hash, active, revoked, last_used_at, created_at, updated_at)
VALUES
(2, 'Demo API Key', '5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8', true, false, EXTRACT(EPOCH FROM NOW() - INTERVAL '1 day'), EXTRACT(EPOCH FROM NOW() - INTERVAL '10 days'), EXTRACT(EPOCH FROM NOW() - INTERVAL '10 days'));