-- Insert default user roles with permissions
-- Using INSERT OR IGNORE to make seeds idempotent (safe to run multiple times)
INSERT OR IGNORE INTO user_roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES 
    ('admin', strftime('%s', 'now'), strftime('%s', 'now'), FALSE, '["is_admin"]'),
    ('user', strftime('%s', 'now'), strftime('%s', 'now'), TRUE, '["can_view_user_self", "can_update_user_self", "can_create_email_self", "can_delete_email_self"]');