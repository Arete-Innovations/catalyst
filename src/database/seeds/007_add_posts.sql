INSERT INTO posts (user_id, title, content, public, created_at, updated_at) VALUES
(2, 'Welcome to Catalyst Framework', 'This is a sample post created by the seed script. Catalyst is a powerful Rust web framework designed for building modern web applications.', true, EXTRACT(EPOCH FROM NOW() - INTERVAL '7 days'), EXTRACT(EPOCH FROM NOW() - INTERVAL '7 days')),
(2, 'API Keys in Catalyst', 'Catalyst provides a robust API key system for securing your application APIs. You can create, manage, and revoke API keys easily through the API Dashboard.', true, EXTRACT(EPOCH FROM NOW() - INTERVAL '5 days'), EXTRACT(EPOCH FROM NOW() - INTERVAL '5 days')),
(2, 'Private Post Example', 'This is a private post that can only be accessed by the owner or through an authenticated API call.', false, EXTRACT(EPOCH FROM NOW() - INTERVAL '3 days'), EXTRACT(EPOCH FROM NOW() - INTERVAL '3 days'))
ON CONFLICT DO NOTHING;
