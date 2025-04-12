INSERT INTO api_key_logs (api_key_id, request_method, request_path, request_ip, response_status, created_at) VALUES
(1, 'GET', '/api/v1/posts', '192.168.1.101', 200, EXTRACT(EPOCH FROM NOW() - INTERVAL '5 days')),
(1, 'GET', '/api/v1/posts/1', '192.168.1.101', 200, EXTRACT(EPOCH FROM NOW() - INTERVAL '5 days')),
(1, 'POST', '/api/v1/posts', '192.168.1.102', 201, EXTRACT(EPOCH FROM NOW() - INTERVAL '4 days')),
(1, 'GET', '/api/v1/user', '192.168.1.103', 200, EXTRACT(EPOCH FROM NOW() - INTERVAL '3 days')),
(1, 'PUT', '/api/v1/posts/2', '192.168.1.103', 200, EXTRACT(EPOCH FROM NOW() - INTERVAL '2 days')),
(1, 'GET', '/api/v1/metrics', '192.168.1.104', 403, EXTRACT(EPOCH FROM NOW() - INTERVAL '1 day')),
(1, 'DELETE', '/api/v1/posts/3', '192.168.1.104', 401, EXTRACT(EPOCH FROM NOW() - INTERVAL '1 day'));