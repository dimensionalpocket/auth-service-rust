-- Insert default user roles
-- Using INSERT OR IGNORE to make seeds idempotent (safe to run multiple times)
INSERT OR IGNORE INTO user_roles (name, created_ts, is_default) VALUES 
    ('admin', strftime('%s', 'now'), FALSE),
    ('user', strftime('%s', 'now'), TRUE);