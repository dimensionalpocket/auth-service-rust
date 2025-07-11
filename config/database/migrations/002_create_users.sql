-- Create users table
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    created_ts INTEGER NOT NULL,
    updated_ts INTEGER NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE,
    role_id INTEGER NOT NULL REFERENCES user_roles(id) ON DELETE RESTRICT,
    password_hash TEXT NOT NULL,
    metadata_json TEXT
);

-- Index for name lookups (uuid already has automatic index from UNIQUE constraint)
CREATE INDEX idx_users_name ON users(name);