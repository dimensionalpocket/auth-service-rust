# Replace DpsAuthApiBuilder with DpsConfig

## Overview

This plan refactors the `DpsAuthApi` initialization to use the external `DpsConfig` struct instead of the custom `DpsAuthApiBuilder` pattern. The goal is to standardize configuration management across the DPS ecosystem while maintaining clear error handling and validation.

## Current Architecture

### DpsAuthApiBuilder (to be removed)
- Custom builder pattern with optional fields
- Properties: `port`, `sqlite_file_path`, `session_secret`, `cookie_domain`, `insecure_cookie`, `development_mode`, `database_pool_size`
- Validates configuration in `build()` method
- Returns `DpsAuthApi` instance

### ResolvedServerConfig (to be renamed)
- Internal struct containing resolved configuration values
- All fields are non-optional (resolved with defaults in builder)
- Stored in `DpsAuthApi.config`

### Current Usage Pattern
```rust
let server = DpsAuthApi::new()
    .port(3000)
    .sqlite_file_path("data/production.db")
    .session_secret(your_32_byte_secret)
    .cookie_domain(".yourdomain.com")
    .insecure_cookie(false)
    .development_mode(false)
    .database_pool_size(10)
    .build()?;
```

## New Architecture

### DpsConfig (external crate)
Properties available from DpsConfig that map to current builder:

| DpsAuthApiBuilder Property | DpsConfig Property | DpsConfig Getter | Default |
|----------------------------|-------------------|------------------|---------|
| `port` | `auth_api_port` | `get_auth_api_port()` | `None` |
| `sqlite_file_path` | `auth_api_sqlite_main_file_path` | `get_auth_api_sqlite_main_file_path()` | `"data/main-development.db"` |
| `session_secret` | `auth_api_session_secret` | `get_auth_api_session_secret()` | `None` |
| `cookie_domain` | `domain` + `api_subdomain` | `get_api_domain()` | `"api.dps.localhost"` |
| `insecure_cookie` | `auth_api_insecure_cookie` | `get_auth_api_insecure_cookie()` | `false` |
| `development_mode` | `development_mode` | `get_development_mode()` | `false` |
| `database_pool_size` | `auth_api_sqlite_main_pool_size` | `get_auth_api_sqlite_main_pool_size()` | `1` |

**Note**: `get_auth_api_session_secret_bytes()` returns `Option<Vec<u8>>` which is perfect for our needs.

**DpsConfig Version**: This plan requires `dps-config` version `0.3.0` or higher to access the new properties.

### Breaking Changes in DpsConfig

The DpsConfig crate has a breaking change that affects this migration:
- **RENAMED**: `auth_api_sqlite_file_path` → `auth_api_sqlite_main_file_path`
- **NEW**: `auth_api_insecure_cookie` property added
- **NEW**: `auth_api_sqlite_main_pool_size` property added

### DpsAuthApiConfig (renamed from ResolvedServerConfig)
Internal struct with resolved, validated configuration:

```rust
#[derive(Debug, Clone)]
pub(crate) struct DpsAuthApiConfig {
    pub port: u16,
    pub sqlite_main_file_path: String,
    pub sqlite_main_pool_size: u32,
    pub session_secret: Vec<u8>,
    pub cookie_domain: String,
    pub insecure_cookie: bool,
    pub development_mode: bool,
}
```

**Note**: All fields are non-optional. The field name `sqlite_main_file_path` matches the DpsConfig property name.

### DpsAuthApi Struct
No changes to struct definition:

```rust
pub struct DpsAuthApi {
    pub(crate) config: DpsAuthApiConfig,
}
```

### New Usage Pattern
```rust
use dps_config::DpsConfig;
use dps_auth_api::DpsAuthApi;

// Option 1: Use environment variables (DpsConfig::new() auto-loads)
let config = DpsConfig::new();
let server = DpsAuthApi::new(config)?;
server.start().await?;

// Option 2: Override specific values
let mut config = DpsConfig::new();
config.set_auth_api_port(Some(8080));
config.set_domain("example.com");
config.set_auth_api_sqlite_main_pool_size(10);
let server = DpsAuthApi::new(config)?;
server.start().await?;
```

## Implementation Plan

### Phase 1: Property Mapping Analysis ✓

**Status**: Complete (documented above)

All properties now map cleanly from `DpsAuthApiBuilder` to `DpsConfig`:
- ✅ `port` → `auth_api_port`
- ✅ `sqlite_file_path` → `auth_api_sqlite_main_file_path` (renamed)
- ✅ `session_secret` → `auth_api_session_secret`
- ✅ `cookie_domain` → computed from `get_api_domain()`
- ✅ `insecure_cookie` → `auth_api_insecure_cookie` (newly added)
- ✅ `development_mode` → `development_mode`
- ✅ `database_pool_size` → `auth_api_sqlite_main_pool_size` (newly added)

### Phase 2: Create DpsAuthApiConfig

**Files to modify**: [`src/dps_auth_api.rs`](src/dps_auth_api.rs)

1. Rename `ResolvedServerConfig` to `DpsAuthApiConfig`
2. Rename `sqlite_file_path` field to `sqlite_main_file_path` to match DpsConfig naming
3. In `DpsAuthApiConfig`, rename `database_pool_size` to `sqlite_main_pool_size` and change its type from `Option<u32>` to `u32` (non-optional with default)

**Code changes**:
```rust
// Before
pub(crate) struct ResolvedServerConfig {
    pub port: u16,
    pub sqlite_file_path: String,
    pub session_secret: Vec<u8>,
    pub cookie_domain: String,
    pub insecure_cookie: bool,
    pub development_mode: bool,
    pub database_pool_size: Option<u32>,
}

// After
pub(crate) struct DpsAuthApiConfig {
    pub port: u16,
    pub sqlite_main_file_path: String,
    pub session_secret: Vec<u8>,
    pub cookie_domain: String,
    pub insecure_cookie: bool,
    pub development_mode: bool,
    pub sqlite_main_pool_size: u32,
}
```

### Phase 3: Refactor DpsAuthApi Struct

**Files to modify**: [`src/dps_auth_api.rs`](src/dps_auth_api.rs)

Update struct definition to use renamed config:

**Code changes**:
```rust
// Before
pub struct DpsAuthApi {
    pub(crate) config: ResolvedServerConfig,
}

// After
pub struct DpsAuthApi {
    pub(crate) config: DpsAuthApiConfig,
}
```

### Phase 4: Implement New DpsAuthApi::new()

**Files to modify**: [`src/dps_auth_api.rs`](src/dps_auth_api.rs)

Replace the current `new()` method that returns `DpsAuthApiBuilder` with a method that accepts `DpsConfig`:

**Code changes**:
```rust
impl DpsAuthApi {
    /// Create a new DpsAuthApi instance from DpsConfig
    /// 
    /// This validates the configuration and returns an error if required fields are missing
    /// or invalid. The server will not start if configuration is invalid.
    /// 
    /// # Errors
    /// 
    /// Returns `DpsAuthApiError::MissingRequiredConfig` if session_secret is not set
    /// Returns `DpsAuthApiError::InvalidSecretLength` if session_secret is not exactly 32 bytes
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use dps_config::DpsConfig;
    /// use dps_auth_api::DpsAuthApi;
    /// 
    /// let config = DpsConfig::new();
    /// let server = DpsAuthApi::new(config)?;
    /// ```
    pub fn new(dps_config: dps_config::DpsConfig) -> Result<Self, DpsAuthApiError> {
        // Extract session secret (required)
        let session_secret = dps_config
            .get_auth_api_session_secret_bytes()
            .ok_or(DpsAuthApiError::MissingRequiredConfig {
                field: "auth_api_session_secret".to_string(),
            })?;

        // Validate session secret length
        if session_secret.len() != 32 {
            return Err(DpsAuthApiError::InvalidSecretLength {
                actual: session_secret.len(),
                expected: 32,
            });
        }

        // Build resolved config with defaults from DpsConfig
        let config = DpsAuthApiConfig {
            port: dps_config.get_auth_api_port().unwrap_or(3000),
            sqlite_main_file_path: dps_config.get_auth_api_sqlite_main_file_path(),
            session_secret,
            cookie_domain: format!(".{}", dps_config.get_api_domain()),
            insecure_cookie: dps_config.get_auth_api_insecure_cookie(),
            development_mode: dps_config.get_development_mode(),
            sqlite_main_pool_size: dps_config.get_auth_api_sqlite_main_pool_size(),
        };

        Ok(DpsAuthApi { config })
    }
    
    // Keep all other methods as-is (start, create_app, etc.)
}
```

### Phase 5: Update Method Implementations

**Files to modify**: [`src/dps_auth_api.rs`](src/dps_auth_api.rs)

Update [`initialize_database()`](src/dps_auth_api.rs:89-96) to use `u32` instead of `Option<u32>`:

**Code changes**:
```rust
// Before (line 90-93)
crate::database::Database::new_with_pool_size(
    &self.config.sqlite_file_path,
    self.config.database_pool_size,
)

// After (sqlite_main_pool_size is now always Some(value))
crate::database::Database::new_with_pool_size(
    &self.config.sqlite_main_file_path,
    Some(self.config.sqlite_main_pool_size),
)
```

### Phase 6: Update run_local_server.rs

**Files to modify**: [`scripts/run_local_server.rs`](scripts/run_local_server.rs)

Replace manual configuration with DpsConfig:

**Code changes**:
```rust
// Before
use dps_auth_api::utils::get_secret_from_env::get_secret_from_env;
use dps_auth_api::DpsAuthApi;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dps_auth_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Read configuration from environment variables
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");
    
    let sqlite_file_path = 
        env::var("DPS_AUTH_SQLITE_FILE").unwrap_or_else(|_| "data/development.db".to_string());
    
    let session_secret = get_secret_from_env("DPS_AUTH_SECRET_KEY", 32)
        .expect("DPS_AUTH_SECRET_KEY environment variable is required");
    
    let cookie_domain = 
        env::var("DPS_AUTH_COOKIE_DOMAIN").unwrap_or_else(|_| ".api.dps.localhost".to_string());
    
    let insecure_cookie = env::var("DPS_AUTH_INSECURE_COOKIE").is_ok();
    
    let development_mode = env::var("DPS_AUTH_ENV").unwrap_or_default() == "development";
    
    let server = DpsAuthApi::new()
        .port(port)
        .sqlite_file_path(sqlite_file_path)
        .session_secret(session_secret)
        .cookie_domain(cookie_domain)
        .insecure_cookie(insecure_cookie)
        .development_mode(development_mode)
        .build()?;
    
    Ok(server.start().await?)
}

// After
use dps_auth_api::DpsAuthApi;
use dps_config::DpsConfig;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dps_auth_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // DpsConfig::new() automatically loads from environment variables
    let config = DpsConfig::new();
    let server = DpsAuthApi::new(config)?;
    Ok(server.start().await?)
}
```

**Environment Variable Migration Notes**:
- `DPS_AUTH_SQLITE_FILE` → `DPS_AUTH_API_SQLITE_MAIN_FILE_PATH` ⚠️ **BREAKING**
- `DPS_AUTH_SECRET_KEY` → `DPS_AUTH_API_SESSION_SECRET` ⚠️ **BREAKING**
- `DPS_AUTH_COOKIE_DOMAIN` → derived from `DPS_DOMAIN` + `DPS_API_SUBDOMAIN` ⚠️ **BREAKING**
- `DPS_AUTH_INSECURE_COOKIE` → `DPS_AUTH_API_INSECURE_COOKIE` (now uses "Y" instead of presence)
- `DPS_AUTH_ENV` → `DPS_DEVELOPMENT_MODE` (now uses "Y" instead of "development")

### Phase 7: Remove DpsAuthApiBuilder

**Files to modify/delete**:
- Delete [`src/dps_auth_api_builder.rs`](src/dps_auth_api_builder.rs)
- Update [`src/lib.rs`](src/lib.rs) to remove export

**Code changes in lib.rs**:
```rust
// Before
pub mod database;
pub mod dps_auth_api;
pub mod dps_auth_api_builder;
// ... other modules ...

pub use dps_auth_api::{DpsAuthApi, DpsAuthApiError};
pub use dps_auth_api_builder::DpsAuthApiBuilder;

// After
pub mod database;
pub mod dps_auth_api;
// ... other modules ...

pub use dps_auth_api::{DpsAuthApi, DpsAuthApiError};
```

Remove this line from [`src/dps_auth_api.rs`](src/dps_auth_api.rs:1):
```rust
use crate::dps_auth_api_builder::DpsAuthApiBuilder;
```

### Phase 8: Update All Tests

**Files to modify**: [`src/dps_auth_api.rs`](src/dps_auth_api.rs) (test modules)

Replace all builder usage in tests with direct DpsAuthApi::new() calls:

**Pattern to find**: `DpsAuthApiBuilder::default()...build()`

**Example replacement**:
```rust
// Before
let secret = vec![1u8; 32];
let server = DpsAuthApiBuilder::default()
    .session_secret(secret)
    .sqlite_file_path(db_path)
    .build()
    .unwrap();

// After
let mut config = DpsConfig::new();
config.set_auth_api_session_secret(Some(
    std::str::from_utf8(&[1u8; 32]).unwrap()
));
config.set_auth_api_sqlite_main_file_path(db_path);
let server = DpsAuthApi::new(config).unwrap();
```

**Test count**: Approximately 20+ test functions need updating based on search results.

**Key testing considerations**:
1. Session secret must be valid UTF-8 for DpsConfig (it stores as String internally)
2. Use `set_auth_api_sqlite_main_file_path()` not `set_auth_api_sqlite_file_path()`
3. Pool size is now `u32` not `Option<u32>`

### Phase 9: Update README.md

**Files to modify**: [`README.md`](README.md)

Update usage examples to reflect new API:

**Code changes**:
```markdown
## Usage

### Minimal Example (with defaults)

```rust
use dps_auth_api::DpsAuthApi;
use dps_config::DpsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // DpsConfig loads from environment variables automatically
    // Requires DPS_AUTH_API_SESSION_SECRET to be set
    let config = DpsConfig::new();
    let server = DpsAuthApi::new(config)?;
    
    server.start().await
}
```

### Complete Configuration Example

```rust
use dps_auth_api::DpsAuthApi;
use dps_config::DpsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = DpsConfig::new();
    
    // Override defaults
    config.set_auth_api_port(Some(3000));
    config.set_auth_api_sqlite_main_file_path("data/production.db");
    config.set_auth_api_session_secret(Some("your-32-byte-secret-here!!!!"));
    config.set_domain("yourdomain.com");
    config.set_auth_api_insecure_cookie(false);
    config.set_development_mode(false);
    config.set_auth_api_sqlite_main_pool_size(10);
    
    let server = DpsAuthApi::new(config)?;
    
    server.start().await
}
```

### With Logging Example

```rust
use dps_auth_api::DpsAuthApi;
use dps_config::DpsConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dps_auth_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = DpsConfig::new();
    let server = DpsAuthApi::new(config)?;

    server.start().await
}
```

Update configuration section:

````markdown
## Configuration

The `DpsAuthApi` uses `DpsConfig` from the `dps-config` crate for configuration management.

### Environment Variables

All configuration is loaded from environment variables via `DpsConfig::new()`:

- `DPS_AUTH_API_PORT`: Server port (default: 3000 if not set)
- `DPS_AUTH_API_SQLITE_MAIN_FILE_PATH`: SQLite database file path (default: "data/main-development.db")
- `DPS_AUTH_API_SQLITE_MAIN_POOL_SIZE`: Database connection pool size (default: 1)
- `DPS_AUTH_API_SESSION_SECRET`: 32-byte secret for session encryption (required)
- `DPS_DOMAIN`: Base domain (default: "dps.localhost")
- `DPS_API_SUBDOMAIN`: API subdomain (default: "api")
- `DPS_AUTH_API_INSECURE_COOKIE`: Set to "Y" to enable insecure cookies (default: false)
- `DPS_DEVELOPMENT_MODE`: Set to "Y" to enable development features like GraphQL playground (default: false)

The cookie domain is automatically derived as `.{api_subdomain}.{domain}` (e.g., ".api.dps.localhost").

### Manual Configuration

You can also set configuration programmatically:

```rust
let mut config = DpsConfig::new();
config.set_auth_api_port(Some(8080));
config.set_domain("example.com");
config.set_auth_api_sqlite_main_pool_size(10);
let server = DpsAuthApi::new(config)?;
```

All configuration options except `auth_api_session_secret` have sensible defaults and are optional.
````

**Update local development section**:
````markdown
## Local Development

For local development, use the provided script:

```bash
# Set environment variables in .env file
cargo run --bin run_local_server
```

The `run_local_server` binary uses environment variables for configuration:
- `DPS_AUTH_API_SESSION_SECRET`: 32-byte secret (required)
- `DPS_AUTH_API_SQLITE_MAIN_FILE_PATH`: SQLite database file path (optional, defaults to "data/main-development.db")
- `DPS_AUTH_API_PORT`: Server port (optional, defaults to 3000)
- `DPS_DOMAIN`: Base domain (optional, defaults to "dps.localhost")
- `DPS_API_SUBDOMAIN`: API subdomain (optional, defaults to "api")
- `DPS_AUTH_API_INSECURE_COOKIE`: Set to "Y" to enable insecure cookies (optional)
- `DPS_DEVELOPMENT_MODE`: Set to "Y" to enable development mode (optional)
- `DPS_AUTH_API_SQLITE_MAIN_POOL_SIZE`: Database pool size (optional, defaults to 1)

**Note**: When using the library directly in your code, use the `DpsAuthApi::new(config)` pattern instead.
````

### Phase 10: Update Cargo.toml

**Files to modify**: [`Cargo.toml`](Cargo.toml)

Update `dps-config` dependency to version `0.3.0` which includes the new properties:

**Code changes**:
```toml
# Before
dps-config = { git = "https://github.com/dimensionalpocket/dps-config-rs", tag = "0.2.0" }

# After
dps-config = { git = "https://github.com/dimensionalpocket/dps-config-rs", tag = "0.3.0" }
```

This version includes:
- `auth_api_insecure_cookie` property
- `auth_api_sqlite_main_pool_size` property
- Breaking change: `auth_api_sqlite_file_path` → `auth_api_sqlite_main_file_path`

## Migration Guide for Users

### Breaking Changes

1. **Builder pattern removed**: `DpsAuthApi::new()` no longer returns a builder
2. **Direct instantiation**: Must pass `DpsConfig` to `DpsAuthApi::new(config)`
3. **Environment variable changes**:
   - `DPS_AUTH_SQLITE_FILE` → `DPS_AUTH_API_SQLITE_MAIN_FILE_PATH`
   - `DPS_AUTH_SECRET_KEY` → `DPS_AUTH_API_SESSION_SECRET`
   - `DPS_AUTH_COOKIE_DOMAIN` → computed from `DPS_DOMAIN` + `DPS_API_SUBDOMAIN`
   - `DPS_AUTH_INSECURE_COOKIE` → `DPS_AUTH_API_INSECURE_COOKIE` (now "Y" not just presence)
   - `DPS_AUTH_ENV="development"` → `DPS_DEVELOPMENT_MODE="Y"`
4. **Pool size type**: Now `u32` instead of `Option<u32>` (always has a value)

### Migration Steps

**Before**:
```rust
let server = DpsAuthApi::new()
    .port(3000)
    .sqlite_file_path("data/production.db")
    .session_secret(secret)
    .cookie_domain(".example.com")
    .insecure_cookie(false)
    .development_mode(false)
    .database_pool_size(10)
    .build()?;
```

**After**:
```rust
let mut config = DpsConfig::new();
config.set_auth_api_port(Some(3000));
config.set_auth_api_sqlite_main_file_path("data/production.db");
config.set_auth_api_session_secret(Some("your-secret"));
config.set_domain("example.com");
config.set_auth_api_insecure_cookie(false);
config.set_development_mode(false);
config.set_auth_api_sqlite_main_pool_size(10);

let server = DpsAuthApi::new(config)?;
```

**Environment Variables Before**:
```bash
export PORT="3000"
export DPS_AUTH_SQLITE_FILE="data/production.db"
export DPS_AUTH_SECRET_KEY="your-32-byte-secret-here!!!!"
export DPS_AUTH_COOKIE_DOMAIN=".example.com"
export DPS_AUTH_INSECURE_COOKIE=  # Any value enabled it
export DPS_AUTH_ENV="development"
```

**Environment Variables After**:
```bash
export DPS_AUTH_API_PORT="3000"
export DPS_AUTH_API_SQLITE_MAIN_FILE_PATH="data/production.db"
export DPS_AUTH_API_SESSION_SECRET="your-32-byte-secret-here!!!!"
export DPS_DOMAIN="example.com"
export DPS_API_SUBDOMAIN="api"
export DPS_AUTH_API_INSECURE_COOKIE="Y"
export DPS_DEVELOPMENT_MODE="Y"
export DPS_AUTH_API_SQLITE_MAIN_POOL_SIZE="10"
```

## Testing Strategy

### Unit Tests
- Test `DpsAuthApi::new()` with valid DpsConfig
- Test `DpsAuthApi::new()` with missing session_secret
- Test `DpsAuthApi::new()` with invalid secret length (not 32 bytes)
- Test that defaults are applied correctly from DpsConfig
- Test all configuration properties are properly extracted
- Test cookie domain is correctly derived from domain + api_subdomain

### Integration Tests
- Verify server starts with DpsConfig
- Verify all endpoints work correctly
- Verify session middleware uses correct configuration
- Verify GraphQL handler receives correct config values
- Verify database pool size is used correctly

### Test File Updates
All tests in [`src/dps_auth_api.rs`](src/dps_auth_api.rs) need updating (approximately 20+ tests)

## Summary of File Changes

| File | Action | Complexity |
|------|--------|-----------|
| [`Cargo.toml`](Cargo.toml) | Modify (update dps-config version) | Low |
| [`src/dps_auth_api.rs`](src/dps_auth_api.rs) | Modify (struct rename, new(), tests) | High |
| [`src/dps_auth_api_builder.rs`](src/dps_auth_api_builder.rs) | Delete | Low |
| [`src/lib.rs`](src/lib.rs) | Modify (remove export) | Low |
| [`scripts/run_local_server.rs`](scripts/run_local_server.rs) | Modify (use DpsConfig) | Medium |
| [`README.md`](README.md) | Modify (update examples & env vars) | Medium |

**Total**: 6 files to modify, 1 file to delete
