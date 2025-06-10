-- Insert request logs
INSERT INTO api_request_logs (api_key_id, request_method, request_path, request_ip, request_headers, request_content_length, request_content_type, created_at) VALUES
(1, 'GET', '/api/v1/posts', '192.168.1.101', '{"Accept": "application/json", "User-Agent": "Mozilla/5.0"}', NULL, 'application/json', EXTRACT(EPOCH FROM NOW() - INTERVAL '5 days')),
(1, 'GET', '/api/v1/posts/1', '192.168.1.101', '{"Accept": "application/json", "User-Agent": "Mozilla/5.0"}', NULL, 'application/json', EXTRACT(EPOCH FROM NOW() - INTERVAL '5 days')),
(1, 'POST', '/api/v1/posts', '192.168.1.102', '{"Accept": "application/json", "User-Agent": "Mozilla/5.0"}', 256, 'application/json', EXTRACT(EPOCH FROM NOW() - INTERVAL '4 days')),
(1, 'GET', '/api/v1/user', '192.168.1.103', '{"Accept": "application/json", "User-Agent": "Mozilla/5.0"}', NULL, 'application/json', EXTRACT(EPOCH FROM NOW() - INTERVAL '3 days')),
(1, 'PUT', '/api/v1/posts/2', '192.168.1.103', '{"Accept": "application/json", "User-Agent": "Mozilla/5.0"}', 128, 'application/json', EXTRACT(EPOCH FROM NOW() - INTERVAL '2 days')),
(1, 'GET', '/api/v1/metrics', '192.168.1.104', '{"Accept": "application/json", "User-Agent": "Mozilla/5.0"}', NULL, 'application/json', EXTRACT(EPOCH FROM NOW() - INTERVAL '1 day')),
(1, 'DELETE', '/api/v1/posts/3', '192.168.1.104', '{"Accept": "application/json", "User-Agent": "Mozilla/5.0"}', NULL, 'application/json', EXTRACT(EPOCH FROM NOW() - INTERVAL '1 day'))
ON CONFLICT (api_key_id, request_method, request_path) DO UPDATE SET
    request_ip = EXCLUDED.request_ip,
    request_headers = EXCLUDED.request_headers,
    request_content_length = EXCLUDED.request_content_length,
    request_content_type = EXCLUDED.request_content_type,
    created_at = EXCLUDED.created_at;

-- Insert response logs
INSERT INTO api_response_logs (request_log_id, response_status, response_time_ms, response_content_length, response_content_type, response_headers, created_at) VALUES
(1, 200, 42, 1024, 'application/json', '{"Content-Type": "application/json", "Server": "Catalyst"}', EXTRACT(EPOCH FROM NOW() - INTERVAL '5 days')),
(2, 200, 36, 512, 'application/json', '{"Content-Type": "application/json", "Server": "Catalyst"}', EXTRACT(EPOCH FROM NOW() - INTERVAL '5 days')),
(3, 201, 78, 256, 'application/json', '{"Content-Type": "application/json", "Server": "Catalyst"}', EXTRACT(EPOCH FROM NOW() - INTERVAL '4 days')),
(4, 200, 25, 384, 'application/json', '{"Content-Type": "application/json", "Server": "Catalyst"}', EXTRACT(EPOCH FROM NOW() - INTERVAL '3 days')),
(5, 200, 54, 128, 'application/json', '{"Content-Type": "application/json", "Server": "Catalyst"}', EXTRACT(EPOCH FROM NOW() - INTERVAL '2 days')),
(6, 403, 18, 64, 'application/json', '{"Content-Type": "application/json", "Server": "Catalyst"}', EXTRACT(EPOCH FROM NOW() - INTERVAL '1 day')),
(7, 401, 15, 96, 'application/json', '{"Content-Type": "application/json", "Server": "Catalyst"}', EXTRACT(EPOCH FROM NOW() - INTERVAL '1 day'))
ON CONFLICT (request_log_id) DO UPDATE SET
    response_status = EXCLUDED.response_status,
    response_time_ms = EXCLUDED.response_time_ms,
    response_content_length = EXCLUDED.response_content_length,
    response_content_type = EXCLUDED.response_content_type,
    response_headers = EXCLUDED.response_headers,
    created_at = EXCLUDED.created_at;
