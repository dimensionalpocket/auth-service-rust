-- Database Schema Dump
-- Generated automatically by dps-auth-api-migrate

CREATE TABLE _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    checksum BLOB NOT NULL,
    execution_time BIGINT NOT NULL
);

CREATE TABLE user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_ts INTEGER NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    permissions_json TEXT NOT NULL DEFAULT '[]'
);

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

CREATE INDEX idx_users_name ON users(name);

