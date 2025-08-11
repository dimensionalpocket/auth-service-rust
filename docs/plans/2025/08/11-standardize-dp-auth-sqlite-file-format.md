# Standardize DP_AUTH_SQLITE_FILE Environment Variable and API Parameter Format

**Date**: 2025-08-11@14:09  
**Status**: Draft

## Problem Statement

The `DP_AUTH_SQLITE_FILE` environment variable and the `ServerConfig::database_url` API parameter are currently used inconsistently across the codebase. Some places expect them to contain the "sqlite:" prefix, while others expect just the file path. This creates confusion and inconsistency in configuration for both environment variables and programmatic API usage.

## Current State Analysis

### Current Usage Patterns

1. **With "sqlite:" prefix**:
   - `scripts/run_local_server.rs`: Default value `"sqlite:data/development.db"`
   - `README.md`: Documentation shows `"sqlite:data/development.db"` as default
   - `README.md`: Example usage shows `"sqlite:data/production.db"`
   - `src/config.rs`: `ServerConfig::database_url` field expects full URL with prefix
   - `README.md`: API examples show `"sqlite:data/production.db".to_string()`

2. **Without "sqlite:" prefix**:
   - `src/migration_config.rs`: Expects just the file path, then adds "sqlite:" prefix in `database_url()` method
   - `README.md`: Production examples show just file paths like `"./production.db"` and `"data/development.db"`

### Key Files Affected

1. **scripts/run_local_server.rs** (line 16):
   ```rust
   let database_url = env::var("DP_AUTH_SQLITE_FILE")
     .unwrap_or_else(|_| "sqlite:data/development.db".to_string());
   ```

2. **src/migration_config.rs** (lines 10, 58, 74-75):
   ```rust
   #[arg(long, env = "DP_AUTH_SQLITE_FILE")]
   pub sqlite_file: Option<PathBuf>,
   
   // Default fallback
   .unwrap_or_else(|| PathBuf::from("data/development.db"));
   
   // Method that adds prefix
   pub fn database_url(&self) -> String {
     format!("sqlite:{}", self.sqlite_file.display())
   }
   ```

3. **README.md** (lines 65, 116, 152):
   - Documentation inconsistency between examples

## Proposed Solution

**Standardize both `DP_AUTH_SQLITE_FILE` environment variable and `ServerConfig` API to contain ONLY the file path, without the "sqlite:" prefix.**

### Changes Summary

1. **Environment Variable**: `DP_AUTH_SQLITE_FILE` should contain only the file path
2. **API Parameter**: Change `ServerConfig::database_url` to `ServerConfig::sqlite_file_path`
3. **Internal Handling**: Add "sqlite:" prefix internally when creating database connections

### Rationale

1. **Complete Consistency**: Both environment variables and API parameters use the same format
2. **Simplicity**: Users don't need to remember when to add the "sqlite:" prefix
3. **Flexibility**: The prefix is an implementation detail that should be handled internally
4. **Better UX**: More intuitive for users to specify just the file path
5. **Clearer Intent**: Parameter name `sqlite_file_path` makes it obvious it's a file path, not a URL
6. **Future-Proof**: Easier to extend for other database types if needed

## Implementation Plan

### Phase 1: Update ServerConfig API

**File**: `src/config.rs`

**Changes**:
1. Rename `database_url` field to `sqlite_file_path`
2. Update constructor parameter name
3. Add internal method to generate database URL
4. Update all references

```rust
#[derive(Debug, Clone)]
pub struct ServerConfig {
  pub port: u16,
  pub sqlite_file_path: String, // Changed from database_url
  pub session_secret: Vec<u8>, // 32-byte secret
  pub cookie_domain: String,
  pub insecure_cookie: bool,
  pub development_mode: bool,
}

impl ServerConfig {
  pub fn new(
    port: u16,
    sqlite_file_path: String, // Changed from database_url
    session_secret: Vec<u8>,
    cookie_domain: String,
    insecure_cookie: bool,
    development_mode: bool,
  ) -> Result<Self, ConfigError> {
    // Validate session secret is exactly 32 bytes
    if session_secret.len() != 32 {
      return Err(ConfigError::InvalidSecretLength {
        actual: session_secret.len(),
        expected: 32,
      });
    }

    Ok(Self {
      port,
      sqlite_file_path,
      session_secret,
      cookie_domain,
      insecure_cookie,
      development_mode,
    })
  }

  // Note: database_url() method removed - no longer needed
  // Database::new() now handles the sqlite: prefix internally
}
```

### Phase 2: Update Database Constructor

**File**: `src/database/mod.rs`

**Changes**:
1. Update `Database::new()` to accept `sqlite_file_path` instead of `database_url`
2. Handle "sqlite:" prefix internally

```rust
impl Database {
  pub async fn new(sqlite_file_path: &str) -> Result<Self, sqlx::Error> {
    // Generate database URL internally
    let database_url = format!("sqlite:{}", sqlite_file_path);
    
    // Create database if it doesn't exist
    if !Sqlite::database_exists(&database_url).await.unwrap_or(false) {
      Sqlite::create_database(&database_url).await?;
    }

    let pool = SqlitePool::connect(&database_url).await?;

    // Configure SQLite settings after connection
    Self::configure_sqlite(&pool).await?;
    
    Ok(Self { pool })
  }
  
  // ... rest of implementation
}
```

### Phase 3: Update Library Entry Point

**File**: `src/lib.rs`

**Changes**:
```rust
// Current (line 32)
let _database = database::Database::new(&config.database_url).await?;

// New
let _database = database::Database::new(&config.sqlite_file_path).await?;
```

### Phase 4: Update Local Server Script

**File**: `scripts/run_local_server.rs`

**Changes**:
```rust
// Current (line 15-16)
let database_url =
  env::var("DP_AUTH_SQLITE_FILE").unwrap_or_else(|_| "sqlite:data/development.db".to_string());

// New
let sqlite_file_path =
  env::var("DP_AUTH_SQLITE_FILE").unwrap_or_else(|_| "data/development.db".to_string());

// Update ServerConfig::new call (line 28-35)
let config = ServerConfig::new(
  port,
  sqlite_file_path, // Changed from database_url
  session_secret,
  cookie_domain,
  insecure_cookie,
  development_mode,
)?;
```

### Phase 5: Update Migration Script

**File**: `config/scripts/dp_auth_migrate.rs`

**Changes**:
```rust
// Current (line 70)
let database = Database::new(&config.database_url()).await?;

// New (Database::new now expects file path, not database URL)
let database = Database::new(&config.sqlite_file.display().to_string()).await?;
```

**Note**: Since `Database::new()` now accepts a file path parameter (from Phase 2), we pass the `sqlite_file` field directly. The `Database::new()` method will handle adding the "sqlite:" prefix internally.

### Phase 6: Update Documentation

**File**: `README.md`

**Changes**:
1. **Line 32**: Update example usage
   ```rust
   // Current
   "sqlite:data/production.db".to_string(), // database_url
   
   // New
   "data/production.db".to_string(), // sqlite_file_path
   ```

2. **Line 47**: Update ServerConfig parameter description
   ```markdown
   // Current
   - `database_url`: SQLite database file path (String)
   
   // New
   - `sqlite_file_path`: SQLite database file path (String)
   ```

3. **Line 65**: Update environment variable description
   ```markdown
   // Current
   - `DP_AUTH_SQLITE_FILE`: SQLite database path (optional, defaults to "sqlite:data/development.db")
   
   // New
   - `DP_AUTH_SQLITE_FILE`: SQLite database file path (optional, defaults to "data/development.db")
   ```

4. **Lines 116, 152**: Update examples to use file paths without prefix
   ```bash
   # Current
   export DP_AUTH_SQLITE_FILE="./production.db"
   export DP_AUTH_SQLITE_FILE="data/development.db"
   
   # New (already correct, no changes needed)
   export DP_AUTH_SQLITE_FILE="./production.db"
   export DP_AUTH_SQLITE_FILE="data/development.db"
   ```

### Phase 7: Update Test Files

**File**: `tests/database_integration_tests.rs`

**Changes**:
```rust
// Current (lines 14-15)
let database_url = format!("sqlite:{}", temp_file.path().display());
let pool = SqlitePool::connect(&database_url)

// New
let sqlite_file_path = temp_file.path().display().to_string();
// Update create_test_database function to use Database::new() instead of direct SqlitePool::connect
```

**File**: `src/database/mod.rs` (test helper function)

**Changes**:
```rust
// Current (lines 184-187)
let database_url = format!("sqlite:{}", temp_file.path().display());
let pool = SqlitePool::connect(&database_url)

// New - update create_test_pool to use Database::new()
let sqlite_file_path = temp_file.path().display().to_string();
let database = Database::new(&sqlite_file_path).await.expect("Failed to create test database");
let pool = database.pool;
```

### Phase 8: Remove Obsolete Methods and Verify No Usage

**File**: `src/config.rs`

**Changes**: Remove the `database_url()` method since it's no longer needed:
```rust
// Remove this method entirely
pub fn database_url(&self) -> String {
  format!("sqlite:{}", self.sqlite_file_path)
}
```

**Verification Steps**: Search for any remaining calls to `database_url()` in the codebase:
```bash
# Search for any remaining usage of database_url() method
grep -r "\.database_url()" src/
grep -r "\.database_url()" scripts/
grep -r "\.database_url()" config/
grep -r "\.database_url()" tests/

# Also search for the method definition
grep -r "fn database_url" src/
grep -r "fn database_url" config/
```

**Expected Results**: No matches should be found after all phases are complete - the `database_url()` method should be completely eliminated from the codebase.

**File**: `src/migration_config.rs`

**Changes**: Remove the `database_url()` method entirely since `Database::new()` now handles the prefix:
```rust
// Remove this method entirely
pub fn database_url(&self) -> String {
  format!("sqlite:{}", self.sqlite_file.display())
}
```

**File**: `config/scripts/dp_auth_migrate.rs`

**Changes**: Remove the "Database URL" log line since we're now working with file paths:
```rust
// Current (lines 63-66)
println!("📊 Configuration:");
println!("   SQLite file: {}", config.sqlite_file.display());
println!("   Database URL: {}", config.database_url());
println!("   Skip seeds: {}", config.skip_seeds);

// New (remove database URL line)
println!("📊 Configuration:");
println!("   SQLite file: {}", config.sqlite_file.display());
println!("   Skip seeds: {}", config.skip_seeds);
```

## Files to be Modified

1. **src/config.rs**
   - Rename `database_url` field to `sqlite_file_path`
   - Update constructor parameter
   - Remove `database_url()` method (no longer needed)

2. **src/database/mod.rs**
   - Update `Database::new()` to accept `sqlite_file_path` parameter
   - Handle "sqlite:" prefix internally
   - Update test helper functions

3. **src/lib.rs**
   - Update reference to use `config.sqlite_file_path` directly

4. **scripts/run_local_server.rs**
   - Update environment variable handling
   - Update ServerConfig::new() call

5. **config/scripts/dp_auth_migrate.rs**
   - Update to use `config.sqlite_file` directly instead of `database_url()`

6. **tests/database_integration_tests.rs**
   - Update test functions to use new Database::new() signature

7. **README.md**
   - Update documentation examples and descriptions
   - Update API parameter descriptions

## Code Samples

### Updated run_local_server.rs
```rust
use dp_auth_service::utils::get_secret_from_env::get_secret_from_env;
use dp_auth_service::{start_server, ServerConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file for local development
  dotenvy::dotenv().ok();

  // Read configuration from environment variables
  let port = env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string())
    .parse::<u16>()
    .expect("PORT must be a valid number");

  // Get database file path (no sqlite: prefix needed)
  let sqlite_file_path =
    env::var("DP_AUTH_SQLITE_FILE").unwrap_or_else(|_| "data/development.db".to_string());

  let session_secret = get_secret_from_env("DP_AUTH_SECRET_KEY", 32)
    .expect("DP_AUTH_SECRET_KEY environment variable is required");

  let cookie_domain =
    env::var("DP_AUTH_COOKIE_DOMAIN").unwrap_or_else(|_| ".api.dp-auth.localhost".to_string());

  let insecure_cookie = env::var("DP_AUTH_INSECURE_COOKIE").is_ok();

  let development_mode = env::var("APP_ENV").unwrap_or_default() == "development";

  let config = ServerConfig::new(
    port,
    sqlite_file_path, // Changed parameter name
    session_secret,
    cookie_domain,
    insecure_cookie,
    development_mode,
  )?;

  start_server(config).await
}
```

### Updated README.md Usage Example
```rust
use dp_auth_service::{start_server, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::new(
        3000, // port
        "data/production.db".to_string(), // sqlite_file_path (no prefix needed)
        your_32_byte_secret, // session_secret (Vec<u8>)
        ".yourdomain.com".to_string(), // cookie_domain
        false, // insecure_cookie
        false, // development_mode
    )?;

    start_server(config).await
}
```

## Testing Strategy

1. **Unit Tests**: Verify that the local server script correctly formats the database URL
2. **Integration Tests**: Ensure existing database tests continue to work
3. **Manual Testing**: Test with various environment variable configurations

## Backward Compatibility

This change is **breaking** for both environment variable usage and API usage:

1. **Environment Variable**: Users who currently set `DP_AUTH_SQLITE_FILE` with the "sqlite:" prefix will need to remove it
2. **API Usage**: Applications using `ServerConfig::new()` will need to update the parameter name and remove the "sqlite:" prefix

Since this is version 0.1.0, breaking changes are acceptable and this is the ideal time to standardize the API.

## Migration Guide for Users

### Environment Variable Changes
Users currently setting:
```bash
export DP_AUTH_SQLITE_FILE="sqlite:data/my-database.db"
```

Should change to:
```bash
export DP_AUTH_SQLITE_FILE="data/my-database.db"
```

### API Changes
Applications currently using:
```rust
let config = ServerConfig::new(
    port,
    "sqlite:data/production.db".to_string(), // database_url
    session_secret,
    cookie_domain,
    insecure_cookie,
    development_mode,
)?;
```

Should change to:
```rust
let config = ServerConfig::new(
    port,
    "data/production.db".to_string(), // sqlite_file_path
    session_secret,
    cookie_domain,
    insecure_cookie,
    development_mode,
)?;
```

## Success Criteria

1. ✅ `DP_AUTH_SQLITE_FILE` consistently expects only the file path across all components
2. ✅ `ServerConfig` API uses `sqlite_file_path` parameter instead of `database_url`
3. ✅ `Database::new()` accepts `sqlite_file_path` and handles "sqlite:" prefix internally
4. ✅ Single source of truth for database URL formatting (in `Database` struct)
5. ✅ Documentation is updated and consistent for both environment variables and API usage
6. ✅ All existing tests pass
7. ✅ Local development workflow remains unchanged (except for parameter formats)
8. ✅ Migration tool continues to work correctly
9. ✅ No redundant `database_url()` methods in `ServerConfig`