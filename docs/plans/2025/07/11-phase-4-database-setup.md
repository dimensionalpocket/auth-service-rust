# Phase 4: Database Configuration and User Management

**Date**: 2025-07-11  
**Phase**: 4 of 4  
**Goal**: Configure SQLite database with migrations and implement user management tables

## Overview

This phase implements the database layer for the dp-auth-service. We'll configure SQLite with sqlx, create database migrations for user roles and users tables, and establish the foundation for user management functionality. The implementation follows the project's design patterns with SQL Query objects and comprehensive testing.

## Requirements Analysis

Based on the README specifications and user requirements:
- Configure `sqlx` to use SQLite database in `data/development.db` (filename from environment variable)
- Create migration system stored in `config/database/migrations`
- Implement `user_roles` table first (prerequisite for users table)
- Implement `users` table with foreign key to user_roles
- Support schema dump to `config/database/schema.sql`
- Follow existing service patterns with SQL Query objects in `src/queries`

### Database Schema Requirements

#### user_roles table:
- `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
- `name` (TEXT NOT NULL UNIQUE)
- `created_ts` (INTEGER NOT NULL) - timestamp in seconds since epoch

#### users table:
- `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
- `uuid` (TEXT UNIQUE NOT NULL) - unique identifier for external references
- `created_ts` (INTEGER NOT NULL) - timestamp in seconds since epoch
- `updated_ts` (INTEGER NOT NULL) - timestamp in seconds since epoch
- `name` (TEXT NOT NULL COLLATE NOCASE) - case-insensitive user name
- `role_id` (INTEGER NOT NULL REFERENCES user_roles(id) ON DELETE RESTRICT)
- `password_hash` (TEXT NOT NULL) - Argon2 hash from PasswordService
- `metadata_json` (TEXT) - flexible JSON field for additional user data

## Tasks Breakdown

### 1. Dependencies Setup

Add database-related dependencies to `Cargo.toml`:
```toml
[dependencies]
# Existing dependencies...
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid", "migrate"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
# Existing dev dependencies...
tempfile = "3.0"

# Add migration binary
[[bin]]
name = "migrate_and_dump"
path = "config/scripts/migrate_and_dump.rs"
```

### 2. Database Configuration

#### 2.1 Environment Configuration
- Add database URL to environment variables
- Support for different database files per environment
- Default to `data/development.db` for development

#### 2.2 Database Connection Setup
Create `src/database/mod.rs`:
```rust
use sqlx::{SqlitePool, migrate::MigrateDatabase, Sqlite, Row};
use std::env;

pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self, sqlx::Error> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:data/development.db".to_string());
        
        // Create database if it doesn't exist
        if !Sqlite::database_exists(&database_url).await.unwrap_or(false) {
            Sqlite::create_database(&database_url).await?;
        }
        
        let pool = SqlitePool::connect(&database_url).await?;
        
        // Configure SQLite settings after connection
        Self::configure_sqlite(&pool).await?;
        
        Ok(Database { pool })
    }
    
    /// Configure SQLite settings for optimal performance and data integrity
    async fn configure_sqlite(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        // List of SQLite PRAGMA commands to execute on connection
        // Add more commands to this array as needed for your application
        let pragma_commands = [
            // Enable WAL (Write-Ahead Logging) mode for better concurrency
            // WAL mode allows multiple readers while a writer is active
            "PRAGMA journal_mode = WAL;",
            
            // Enable foreign key constraints enforcement
            // SQLite doesn't enforce foreign keys by default
            "PRAGMA foreign_keys = ON;",
            
            // Set synchronous mode to NORMAL for better performance
            // NORMAL is safer than OFF but faster than FULL
            "PRAGMA synchronous = NORMAL;",
            
            // Additional performance optimizations (commented out for conservative defaults)
            // Uncomment and adjust these based on your specific performance requirements:
            
            // Set cache size to 64MB (negative value = KB, positive = pages)
            // Larger cache improves performance for read-heavy workloads
            // "PRAGMA cache_size = -65536;",
            
            // Enable memory-mapped I/O for better performance
            // Uses 256MB of memory-mapped I/O
            // "PRAGMA mmap_size = 268435456;",
            
            // Set temp store to memory for temporary tables and indices
            // Faster than disk-based temporary storage
            // "PRAGMA temp_store = MEMORY;",
            
            // Optimize for faster writes at the cost of some durability
            // Only use in development; consider removing in production
            // "PRAGMA wal_autocheckpoint = 1000;",
        ];
        
        // Execute each PRAGMA command
        for command in pragma_commands.iter() {
            println!("Executing SQLite configuration: {}", command);
            sqlx::query(command).execute(pool).await?;
        }
        
        // Verify critical settings were applied correctly
        Self::verify_sqlite_config(pool).await?;
        
        println!("✅ SQLite configuration completed successfully");
        Ok(())
    }
    
    /// Verify that critical SQLite settings were applied correctly
    async fn verify_sqlite_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        // Check WAL mode is enabled
        let journal_mode: String = sqlx::query("PRAGMA journal_mode;")
            .fetch_one(pool)
            .await?
            .get(0);
        
        if journal_mode.to_uppercase() != "WAL" {
            eprintln!("⚠️  Warning: WAL mode not enabled, got: {}", journal_mode);
        }
        
        // Check foreign keys are enabled
        let foreign_keys: i64 = sqlx::query("PRAGMA foreign_keys;")
            .fetch_one(pool)
            .await?
            .get(0);
        
        if foreign_keys != 1 {
            eprintln!("⚠️  Warning: Foreign keys not enabled");
        }
        
        println!("✅ SQLite configuration verified");
        Ok(())
    }
    
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./config/database/migrations").run(&self.pool).await
    }
}

/*
ADDING MORE SQLITE CONFIGURATION COMMANDS:

To add more SQLite PRAGMA commands, simply add them to the `pragma_commands` array
in the `configure_sqlite` method above. Here are some additional useful commands:

Performance Tuning:
- "PRAGMA optimize;" - Analyze query patterns and update statistics
- "PRAGMA analysis_limit = 1000;" - Limit ANALYZE command processing time
- "PRAGMA threads = 4;" - Enable multi-threading (SQLite 3.35+)

Security:
- "PRAGMA secure_delete = ON;" - Overwrite deleted data (slower but more secure)
- "PRAGMA cell_size_check = ON;" - Enable additional corruption detection

Debugging:
- "PRAGMA integrity_check;" - Check database integrity (expensive operation)
- "PRAGMA quick_check;" - Faster integrity check

Connection-specific:
- "PRAGMA busy_timeout = 30000;" - Set timeout for locked database (30 seconds)
- "PRAGMA case_sensitive_like = ON;" - Make LIKE operator case-sensitive

Example of adding a new command:
1. Add the command string to the `pragma_commands` array
2. Optionally add verification logic in `verify_sqlite_config`
3. Document the purpose and any trade-offs in comments

Note: Some PRAGMA commands are connection-specific and need to be set for each
connection. For those, consider using SQLx connection options or connection
event handlers instead of this global configuration.
*/
```

### 3. Migration System Setup

#### 3.1 Directory Structure
```
config/
├── database/
│   ├── migrations/
│   │   ├── 001_create_user_roles.sql
│   │   ├── 001_create_user_roles.down.sql
│   │   ├── 002_create_users.sql
│   │   └── 002_create_users.down.sql
│   ├── seeds/
│   │   └── 001_default_roles.sql
│   └── schema.sql (generated by migration script)
└── scripts/
    ├── migrate_and_dump.rs
    └── migrate.sh
```

#### 3.2 Migration 001: Create user_roles table
File: `config/database/migrations/001_create_user_roles.sql`
```sql
-- Create user_roles table
CREATE TABLE user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_ts INTEGER NOT NULL
);
```

Down migration: `config/database/migrations/001_create_user_roles.down.sql`
```sql
DROP TABLE IF EXISTS user_roles;
```

#### 3.3 Migration 002: Create users table
File: `config/database/migrations/002_create_users.sql`
```sql
-- Create users table
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    created_ts INTEGER NOT NULL,
    updated_ts INTEGER NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE,
    role_id INTEGER NOT NULL REFERENCES user_roles(id) ON DELETE RESTRICT,
    password_hash TEXT NOT NULL,
    metadata_json TEXT,
    
    -- Index for name lookups (uuid already has automatic index from UNIQUE constraint)
    INDEX idx_users_name (name)
);
```

Down migration: `config/database/migrations/002_create_users.down.sql`
```sql
DROP TABLE IF EXISTS users;
```

#### 3.4 Seeds System

SQLx doesn't have built-in seeding support, but we can implement a custom seeding mechanism that runs after migrations. Seeds are used for inserting default/reference data that the application needs to function.

**Seed File**: `config/database/seeds/001_default_roles.sql`
```sql
-- Insert default user roles
-- Using INSERT OR IGNORE to make seeds idempotent (safe to run multiple times)
INSERT OR IGNORE INTO user_roles (name, created_ts) VALUES 
    ('admin', strftime('%s', 'now')),
    ('user', strftime('%s', 'now'));
```

**Seeds Implementation** (added to the Database module above):
```rust
// This method is added to the Database impl block shown in section 2.2
pub async fn seed(&self) -> Result<(), Box<dyn std::error::Error>> {
    let seeds_dir = "config/database/seeds";
    
    // Check if seeds directory exists
    if !std::path::Path::new(seeds_dir).exists() {
        println!("No seeds directory found, skipping seeding");
        return Ok(());
    }
    
    // Read all .sql files in seeds directory
    let mut seed_files = fs::read_dir(seeds_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "sql" {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    // Sort files to ensure consistent execution order
    seed_files.sort();
    
    // Execute each seed file
    for seed_file in seed_files {
        let sql_content = fs::read_to_string(&seed_file)?;
        println!("Running seed: {}", seed_file.display());
        
        // Split by semicolon and execute each statement
        for statement in sql_content.split(';') {
            let statement = statement.trim();
            if !statement.is_empty() && !statement.starts_with("--") {
                sqlx::query(statement).execute(&self.pool).await?;
            }
        }
    }
    
    println!("✅ Seeds completed successfully");
    Ok(())
}
```

### 4. SQL Query Objects Implementation

Following the project's design pattern, create SQL Query objects in `src/queries/`:

#### 4.1 Directory Structure
```
src/
├── models/
│   ├── mod.rs
│   ├── user_role.rs
│   └── user.rs
└── queries/
    ├── mod.rs
    ├── user_roles/
    │   ├── mod.rs
    │   ├── get_all_roles.rs
    │   ├── get_role_by_id.rs
    │   └── get_role_by_name.rs
    └── users/
        ├── mod.rs
        ├── create_user.rs
        ├── get_user_by_id.rs
        ├── get_user_by_uuid.rs
```

#### 4.2 Model Definitions

**UserRole Model** (`src/models/user_role.rs`):
```rust
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct UserRole {
    pub id: i64,
    pub name: String,
    pub created_ts: i64,
}
```

**User Model** (`src/models/user.rs`):
```rust
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct User {
    pub id: i64,
    pub uuid: String,
    pub created_ts: i64,
    pub updated_ts: i64,
    pub name: String,
    pub role_id: i64,
    pub password_hash: String,
    pub metadata_json: Option<String>,
}
```

**Models Module** (`src/models/mod.rs`):
```rust
pub mod user_role;
pub mod user;

pub use user_role::UserRole;
pub use user::User;
```

#### 4.3 User Roles Query Objects

**GetAllRolesQuery** (`src/queries/user_roles/get_all_roles.rs`):
```rust
use sqlx::SqlitePool;
use crate::models::UserRole;

pub struct GetAllRolesQuery;

impl GetAllRolesQuery {
    pub async fn run(pool: &SqlitePool) -> Result<Vec<UserRole>, sqlx::Error> {
        sqlx::query_as::<_, UserRole>(
            "SELECT id, name, created_ts FROM user_roles ORDER BY name"
        )
        .fetch_all(pool)
        .await
    }
}
```

**GetRoleByIdQuery** (`src/queries/user_roles/get_role_by_id.rs`):
```rust
use sqlx::SqlitePool;
use crate::models::UserRole;

pub struct GetRoleByIdQuery;

impl GetRoleByIdQuery {
    pub async fn run(pool: &SqlitePool, role_id: i64) -> Result<Option<UserRole>, sqlx::Error> {
        sqlx::query_as::<_, UserRole>(
            "SELECT id, name, created_ts FROM user_roles WHERE id = ?"
        )
        .bind(role_id)
        .fetch_optional(pool)
        .await
    }
}
```

**GetRoleByNameQuery** (`src/queries/user_roles/get_role_by_name.rs`):
```rust
use sqlx::SqlitePool;
use crate::models::UserRole;

pub struct GetRoleByNameQuery;

impl GetRoleByNameQuery {
    pub async fn run(pool: &SqlitePool, name: &str) -> Result<Option<UserRole>, sqlx::Error> {
        sqlx::query_as::<_, UserRole>(
            "SELECT id, name, created_ts FROM user_roles WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(pool)
        .await
    }
}
```

#### 4.4 Users Query Objects

**CreateUserQuery** (`src/queries/users/create_user.rs`):
```rust
use sqlx::SqlitePool;
use crate::models::User;

#[derive(Debug)]
pub struct CreateUserData {
    pub uuid: String,
    pub name: String,
    pub role_id: i64,
    pub password_hash: String,
    pub metadata_json: Option<String>,
}

pub struct CreateUserQuery;

impl CreateUserQuery {
    pub async fn run(pool: &SqlitePool, data: CreateUserData) -> Result<User, sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        
        let result = sqlx::query(
            r#"
            INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&data.uuid)
        .bind(now)
        .bind(now)
        .bind(&data.name)
        .bind(data.role_id)
        .bind(&data.password_hash)
        .bind(&data.metadata_json)
        .execute(pool)
        .await?;
        
        let user_id = result.last_insert_rowid();
        
        // Return the created user
        sqlx::query_as::<_, User>(
            "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
    }
}
```

**GetUserByIdQuery** (`src/queries/users/get_user_by_id.rs`):
```rust
use sqlx::SqlitePool;
use crate::models::User;

pub struct GetUserByIdQuery;

impl GetUserByIdQuery {
    pub async fn run(pool: &SqlitePool, user_id: i64) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }
}
```

**GetUserByUuidQuery** (`src/queries/users/get_user_by_uuid.rs`):
```rust
use sqlx::SqlitePool;
use crate::models::User;

pub struct GetUserByUuidQuery;

impl GetUserByUuidQuery {
    pub async fn run(pool: &SqlitePool, uuid: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json FROM users WHERE uuid = ?"
        )
        .bind(uuid)
        .fetch_optional(pool)
        .await
    }
}
```

### 5. Database Integration in Main Application

#### 5.1 Update main.rs
```rust
use dp_auth_service::database::Database;

#[tokio::main]
async fn main() {
    // Initialize tracing (existing code)
    
    // Initialize database connection (migrations run separately via script)
    let database = Database::new().await.expect("Failed to connect to database");
    
    // Create schema with database pool
    let schema = create_schema_with_database(database.pool.clone());
    
    // Rest of existing code...
}
```

#### 5.2 Update lib.rs
```rust
pub mod graphql;
pub mod handlers;
pub mod middleware;
pub mod services;
pub mod database;
pub mod models;
pub mod queries;
```

### 6. Testing Strategy

#### 6.1 Database Test Utilities
Create `src/database/test_utils.rs`:
```rust
use sqlx::SqlitePool;
use tempfile::NamedTempFile;
use crate::database::Database;

pub async fn create_test_database() -> (SqlitePool, NamedTempFile) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let database_url = format!("sqlite:{}", temp_file.path().display());
    
    let pool = SqlitePool::connect(&database_url).await.expect("Failed to connect to test database");
    
    // Configure SQLite settings (same as production)
    Database::configure_sqlite(&pool).await.expect("Failed to configure SQLite");
    
    // Run migrations
    sqlx::migrate!("./config/database/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    
    (pool, temp_file)
}
```

#### 6.2 Query Object Tests
Each query object will have comprehensive unit tests:

**Test for GetAllRolesQuery** (`src/queries/user_roles/get_all_roles.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_utils::create_test_database;

    #[tokio::test]
    async fn test_get_all_roles_returns_default_roles() {
        let (pool, _temp_file) = create_test_database().await;
        
        let roles = GetAllRolesQuery::run(&pool).await.unwrap();
        
        assert_eq!(roles.len(), 2);
        assert!(roles.iter().any(|r| r.name == "admin"));
        assert!(roles.iter().any(|r| r.name == "user"));
    }

    #[tokio::test]
    async fn test_get_all_roles_ordered_by_name() {
        let (pool, _temp_file) = create_test_database().await;
        
        let roles = GetAllRolesQuery::run(&pool).await.unwrap();
        
        assert_eq!(roles[0].name, "admin");
        assert_eq!(roles[1].name, "user");
    }
}
```

#### 6.3 Integration Tests
Create `tests/database_integration_tests.rs`:
```rust
use dp_auth_service::{
    database::Database,
    queries::{
        user_roles::{GetAllRolesQuery, GetRoleByNameQuery},
        users::{CreateUserQuery, GetUserByUuidQuery, CreateUserData},
    },
    services::PasswordService,
};
use uuid::Uuid;

#[tokio::test]
async fn test_complete_user_creation_flow() {
    // Setup test database
    let database = Database::new().await.unwrap();
    database.migrate().await.unwrap();
    
    // Get user role
    let user_role = GetRoleByNameQuery::run(&database.pool, "user")
        .await
        .unwrap()
        .expect("User role should exist");
    
    // Create password hash
    let password_hash = PasswordService::generate("test_password").unwrap();
    
    // Create user
    let user_uuid = Uuid::new_v4().to_string();
    let create_data = CreateUserData {
        uuid: user_uuid.clone(),
        name: "Test User".to_string(),
        role_id: user_role.id,
        password_hash,
        metadata_json: Some(r#"{"test": true}"#.to_string()),
    };
    
    let created_user = CreateUserQuery::run(&database.pool, create_data)
        .await
        .unwrap();
    
    // Verify user was created correctly
    assert_eq!(created_user.uuid, user_uuid);
    assert_eq!(created_user.name, "Test User");
    assert_eq!(created_user.role_id, user_role.id);
    
    // Verify user can be retrieved by UUID
    let retrieved_user = GetUserByUuidQuery::run(&database.pool, &user_uuid)
        .await
        .unwrap()
        .expect("User should be found");
    
    assert_eq!(retrieved_user.id, created_user.id);
    assert_eq!(retrieved_user.name, created_user.name);
}
```

### 7. Migration and Schema Management

#### 7.1 Migration, Seeding, and Schema Script
Create `config/scripts/migrate_and_dump.rs`:
```rust
use dp_auth_service::database::Database;
use std::env;
use sqlx::SqlitePool;

async fn generate_schema_dump(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    // Query to get all table creation statements
    let tables = sqlx::query_scalar::<_, String>(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    )
    .fetch_all(pool)
    .await?;
    
    // Query to get all index creation statements
    let indexes = sqlx::query_scalar::<_, String>(
        "SELECT sql FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' AND sql IS NOT NULL ORDER BY name"
    )
    .fetch_all(pool)
    .await?;
    
    // Combine all SQL statements
    let mut schema_sql = String::new();
    schema_sql.push_str("-- Database Schema Dump\n");
    schema_sql.push_str("-- Generated automatically by migrate_and_dump.rs\n\n");
    
    // Add table creation statements
    for table_sql in tables {
        schema_sql.push_str(&table_sql);
        schema_sql.push_str(";\n\n");
    }
    
    // Add index creation statements
    for index_sql in indexes {
        schema_sql.push_str(&index_sql);
        schema_sql.push_str(";\n\n");
    }
    
    Ok(schema_sql)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
    println!("Starting database migration, seeding, and schema dump...");
    
    // Initialize database and run migrations
    let database = Database::new().await?;
    database.migrate().await?;
    println!("✅ Migrations completed successfully");
    
    // Run seeds
    database.seed().await?;
    println!("✅ Seeds completed successfully");
    
    // Get database URL for schema dump
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:data/development.db".to_string());
    
    // Extract database file path from URL
    let db_path = database_url.strip_prefix("sqlite:").unwrap_or(&database_url);
    
    // Generate schema dump using pure Rust/sqlx instead of sqlite3 command
    let schema_sql = generate_schema_dump(&database.pool).await?;
    
    // Write schema to file
    std::fs::create_dir_all("config/database")?;
    std::fs::write("config/database/schema.sql", schema_sql)?;
    println!("✅ Schema dumped to config/database/schema.sql");
    
    println!("🎉 Migration, seeding, and schema dump completed successfully!");
    Ok(())
}
```

#### 7.2 Shell Script Wrapper
Create `config/scripts/migrate.sh`:
```bash
#!/bin/bash

# Change to project root directory
cd "$(dirname "$0")/../.."

# Run the migration script (dotenvy handles .env loading)
echo "Running database migrations, seeds, and schema dump..."
mise exec -- cargo run --bin migrate_and_dump

echo "Migration script completed."
```

#### 7.3 Make Script Executable
The script will be made executable as part of the implementation:
```bash
chmod +x config/scripts/migrate.sh
```

Note: This will be done during implementation since file permissions are stored in git.

### 8. Usage Instructions

#### 8.1 Running Migrations and Seeds
To set up the database, run migrations, and seed data:
```bash
# Run migrations, seeds, and generate schema dump
./config/scripts/migrate.sh
```

Note: The script is automatically made executable during implementation.

Or run directly with cargo:
```bash
# Run migration and seeding script directly
mise exec -- cargo run --bin migrate_and_dump
```

#### 8.2 Environment Configuration
Update .env file:
```env
# Database configuration
DATABASE_URL=sqlite:data/development.db

# Existing environment variables...
```

#### 8.3 Environment Documentation
Document environment variables in README:
- `DATABASE_URL` - SQLite database file path (default: `sqlite:data/development.db`)

#### 8.4 Development Workflow
1. **Initial setup**: Run `./config/scripts/migrate.sh` to create database, tables, and seed data
2. **After schema changes**: Run migration script again to update database and schema dump
3. **After seed changes**: Run migration script to apply new seed data (idempotent)
4. **Application startup**: Main application only connects to existing database (no migration/seeding)

Note: The migration script is automatically executable after implementation.

## Files to be Created/Modified

### New Files:
1. `config/database/migrations/001_create_user_roles.sql`
2. `config/database/migrations/001_create_user_roles.down.sql`
3. `config/database/migrations/002_create_users.sql`
4. `config/database/migrations/002_create_users.down.sql`
5. `config/database/seeds/001_default_roles.sql`
6. `config/scripts/migrate_and_dump.rs`
7. `config/scripts/migrate.sh` (will be made executable with `chmod +x`)
8. `src/database/mod.rs`
9. `src/database/test_utils.rs`
10. `src/models/mod.rs`
11. `src/models/user_role.rs`
12. `src/models/user.rs`
13. `src/queries/mod.rs`
14. `src/queries/user_roles/mod.rs`
15. `src/queries/user_roles/get_all_roles.rs`
16. `src/queries/user_roles/get_role_by_id.rs`
17. `src/queries/user_roles/get_role_by_name.rs`
18. `src/queries/users/mod.rs`
19. `src/queries/users/create_user.rs`
20. `src/queries/users/get_user_by_id.rs`
21. `src/queries/users/get_user_by_uuid.rs`
22. `tests/database_integration_tests.rs`

### Modified Files:
1. `Cargo.toml` - Add sqlx and related dependencies, add migrate_and_dump binary
2. `src/lib.rs` - Add database, models, and queries modules
3. `src/main.rs` - Initialize database connection (no migrations)
4. `.env` - Add DATABASE_URL configuration
5. `README.md` - Update Phase 4 status and document migration workflow

## Code Samples Summary

### Key Database Connection Pattern:
```rust
// Database initialization (migrations run separately)
let database = Database::new().await?;
```

### Migration and Seeding Pattern:
```rust
// Separate executable for migrations, seeding, and schema dump
let database = Database::new().await?;
database.migrate().await?;
database.seed().await?;
// + schema dump logic
```

### Query Object Pattern:
```rust
// Non-instantiated static query objects
let roles = GetAllRolesQuery::run(&pool).await?;
let user = GetUserByUuidQuery::run(&pool, &uuid).await?;
```


### Migration Pattern:
```sql
-- Up migration creates table and inserts default data
CREATE TABLE user_roles (...);
INSERT INTO user_roles (name, created_ts) VALUES ('admin', strftime('%s', 'now'));

-- Down migration in separate file
DROP TABLE IF EXISTS user_roles;
```

## Success Criteria

1. ✅ SQLite database configured with environment-based file path
2. ✅ SQLite optimized with WAL mode, foreign keys, and performance settings
3. ✅ Migration system working with up/down migrations
4. ✅ Seeds system working with idempotent default data insertion
5. ✅ user_roles table created with default admin/user roles via seeds
6. ✅ users table created with proper foreign key constraints (enforced)
7. ✅ SQL Query objects implemented following project patterns
8. ✅ Comprehensive unit tests for all query objects
9. ✅ Integration tests demonstrating complete user creation flow
10. ✅ Schema dump generation working
11. ✅ Migration script made executable automatically
12. ✅ Database connection integrated into main application
13. ✅ All existing tests still passing

## Next Steps

After Phase 4 completion, the foundation will be ready for:
- User authentication GraphQL mutations
- JWT token generation and validation
- User management endpoints
- Role-based access control
- Password reset functionality

The database layer provides the essential foundation for all user management features while maintaining the project's established patterns and testing standards.