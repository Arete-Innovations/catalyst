INSERT INTO users (username, email, first_name, last_name, password_hash, role, active, should_change_password, created_at, updated_at) VALUES
('admin', NULL, 'Admin', 'Admin', '$2y$12$iqf7nfi2L4D5dthUGF.Py.uD4Wt7uURKNLeqDg6I5EDTL7QF9XIQ2', 'admin', TRUE, TRUE, EXTRACT(EPOCH FROM NOW()), EXTRACT(EPOCH FROM NOW())),
('user1', NULL, 'User', 'One', '$2y$12$iqf7nfi2L4D5dthUGF.Py.uD4Wt7uURKNLeqDg6I5EDTL7QF9XIQ2', 'user', TRUE, TRUE, EXTRACT(EPOCH FROM NOW()), EXTRACT(EPOCH FROM NOW())),
('user2', NULL, 'User', 'Two', '$2y$12$iqf7nfi2L4D5dthUGF.Py.uD4Wt7uURKNLeqDg6I5EDTL7QF9XIQ2', 'user', TRUE, TRUE, EXTRACT(EPOCH FROM NOW()), EXTRACT(EPOCH FROM NOW()))
ON CONFLICT DO NOTHING;
