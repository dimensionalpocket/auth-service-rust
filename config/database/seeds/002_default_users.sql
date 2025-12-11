-- Insert default users
-- Using INSERT OR IGNORE to make seeds idempotent (safe to run multiple times)
INSERT OR IGNORE INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES 
    (
        '550e8400-e29b-41d4-a716-446655440000', -- Fixed UUID for admin user
        strftime('%s', 'now'), 
        strftime('%s', 'now'), 
        'admin', 
        (SELECT id FROM roles WHERE name = 'admin' LIMIT 1), -- Get admin role_id dynamically
        '$argon2id$v=19$m=4096,t=3,p=1$GchZ1xLZcBURtNwZryiZlw$jXSI8cnf3pyfjIjAZZum1FG2lV9MJtfvS607HJOCo20', -- Hash of "Ch4nG3M3!"
        NULL
    );