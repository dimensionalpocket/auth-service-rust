-- Create user_roles table
CREATE TABLE user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_ts INTEGER NOT NULL,
    updated_ts INTEGER NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    permissions_json TEXT NOT NULL DEFAULT '[]'
);