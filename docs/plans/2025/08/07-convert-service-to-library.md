# Convert dp-auth-service from Service to Library

**Date**: 2025-08-07@23:59

## Overview

Convert the dp-auth-service from a standalone service with environment variable dependencies to a library that can be imported by other crates. The library will expose a `start_server()` function that accepts all configuration parameters directly, eliminating the need for environment variables.

This will be implemented in multiple phases, working from the inside out, with each phase being fully tested before moving to the next.

## Current Environment Variable Dependencies

Based on code analysis, the following environment variables are currently used:

1. **DP_AUTH_SECRET_KEY** - Base64-encoded 32-byte secret for session token encryption
2. **DATABASE_URL** - SQLite database file path (default: "sqlite:data/development.db")
3. **PORT** - Server port (default: "3000")
4. **DP_AUTH_COOKIE_DOMAIN** - Cookie domain (default: ".api.dp-auth.localhost")
5. **DP_AUTH_INSECURE_COOKIE** - Flag for insecure cookies (presence indicates true)
6. **APP_ENV** - Application environment (used for development mode detection)

## Implementation Phases

### Phase 1: Database Module Refactoring ✅ COMPLETED

**Goal**: Update Database struct to accept database_url as constructor parameter instead of reading from environment.

**Current State**: `Database::new()` reads `DATABASE_URL` from environment with fallback to "sqlite:data/development.db"

**Target State**: `Database::new(database_url: &str)` accepts URL as parameter

**Files to Modify**:
- `src/database/mod.rs` - Update constructor signature and implementation
- `src/main.rs` - Update Database::new() call to pass env var value
- `config/scripts/migrate_and_dump.rs` - Update Database::new() call

**Implementation Details**:

Update `src/database/mod.rs`:
```rust
impl Database {
  pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
    // Use provided database_url instead of env::var
    
    // Create database if it doesn't exist
    if !Sqlite::database_exists(database_url)
      .await
      .unwrap_or(false)
    {
      Sqlite::create_database(database_url).await?;
    }

    // Connect to database
    let pool = SqlitePool::connect(database_url).await?;

    Ok(Self { pool })
  }
  
  // Rest of implementation remains the same
}
```

Update `src/main.rs`:
```rust
// Get database URL from environment (temporary - will be moved in later phases)
let database_url = env::var("DATABASE_URL")
  .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

// Initialize database connection with explicit URL
let _database = Database::new(&database_url)
  .await
  .expect("Failed to connect to database");
```

Update `config/scripts/migrate_and_dump.rs`:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file
  dotenvy::dotenv().ok();

  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  println!("Starting database migration, seeding, and schema dump...");

  // Initialize database and run migrations with explicit URL
  let database = Database::new(&database_url).await?;
  // ... rest remains the same
}
```

**Testing**: Run all existing tests to ensure Database functionality is unchanged.

### Phase 2: Session Middleware Refactoring ✅ COMPLETED

**Goal**: Update session middleware to accept secret as parameter instead of using global state.

**Current State**: Uses global `SESSION_SECRET` initialized from `DP_AUTH_SECRET_KEY` environment variable

**Target State**: Session middleware accepts secret as parameter, main.rs reads env var and passes it

**Files to Modify**:
- `src/middleware/session.rs` - Remove global secret, update middleware signature
- `src/main.rs` - Read secret from env and pass to middleware

**Implementation Details**:

Update `src/middleware/session.rs`:
```rust
// Remove these global items:
// static SESSION_SECRET: OnceLock<Vec<u8>> = OnceLock::new();
// pub fn init_session_secret() -> Result<(), SecretError>
// pub fn get_session_secret() -> &'static [u8]

// Add new middleware factory function
pub fn create_session_middleware(
  secret: Vec<u8>
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone {
  move |request: Request, next: Next| {
    let secret = secret.clone();
    Box::pin(async move {
      let session_context = extract_and_validate_session_sync(&request, &secret);
      let mut request = request;
      request.extensions_mut().insert(session_context);
      next.run(request).await
    })
  }
}

// Update this function signature
fn extract_and_validate_session_sync(request: &Request, secret: &[u8]) -> SessionContext {
  // Use provided secret instead of get_session_secret()
  
  // Try header first
  if let Some(token) = extract_token_from_header(request) {
    if let Ok(payload) = DpAuthSessionService::decode_token(&token, secret) {
      return SessionContext::new(Some(payload));
    }
    return SessionContext::new(None);
  }

  // Try cookie if no header
  if let Some(token) = extract_token_from_cookie(request) {
    if let Ok(payload) = DpAuthSessionService::decode_token(&token, secret) {
      return SessionContext::new(Some(payload));
    }
  }

  SessionContext::new(None)
}

// Keep existing helper functions unchanged:
// extract_token_from_header, extract_token_from_cookie
```

Update `src/main.rs`:
```rust
use crate::utils::get_secret_from_env::get_secret_from_env;

#[tokio::main]
async fn main() {
  // ... existing tracing setup ...
  
  // Load environment variables from .env file
  dotenvy::dotenv().ok();

  // Read session secret from environment
  let session_secret = get_secret_from_env("DP_AUTH_SECRET_KEY", 32)
    .expect("Failed to read session secret from DP_AUTH_SECRET_KEY");

  // Get database URL from environment
  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  // Initialize database connection
  let _database = Database::new(&database_url)
    .await
    .expect("Failed to connect to database");

  // Create schema
  let schema = create_schema();

  let app = Router::new()
    .route("/", get(root_handler))
    .route("/health", get(health_handler))
    .route(
      "/graphql",
      get(graphql_get_handler)
        .post(graphql_post_handler)
        .layer(middleware::from_fn(create_session_middleware(session_secret))), // Use new middleware
    )
    // ... rest remains the same
}
```

**Testing**: Run all existing tests, especially session middleware tests, to ensure functionality is unchanged.

### Phase 3: GraphQL Handlers Refactoring

**Goal**: Update GraphQL handlers to accept configuration parameters instead of reading from environment.

**Current State**: `set_session_cookie()` and development mode detection read from environment variables

**Target State**: Functions accept parameters, main.rs reads env vars and passes them

**Files to Modify**:
- `src/handlers/graphql.rs` - Update function signatures to accept config parameters
- `src/main.rs` - Read config from env and pass to handlers

**Implementation Details**:

Update `src/handlers/graphql.rs`:
```rust
// Update function signature
pub fn set_session_cookie(
  response_headers: &Arc<Mutex<HeaderMap>>, 
  token: &str,
  cookie_domain: &str,
  insecure_cookie: bool
) {
  use crate::middleware::session::SESSION_COOKIE_NAME;

  // Use provided parameters instead of env::var calls
  let cookie_value = if insecure_cookie {
    format!("{SESSION_COOKIE_NAME}={token}; Domain={cookie_domain}; Path=/; HttpOnly; SameSite=Lax")
  } else {
    format!("{SESSION_COOKIE_NAME}={token}; Domain={cookie_domain}; Path=/; HttpOnly; Secure; SameSite=Lax")
  };

  // ... rest of implementation
}

// Add config parameter to handlers that need it
pub async fn graphql_post_handler(
  State(schema): State<AppSchema>,
  headers: HeaderMap,
  request: Request<Body>,
  cookie_domain: String,
  insecure_cookie: bool,
  development_mode: bool,
) -> Result<GraphQLResponse, StatusCode> {
  // Pass config parameters to resolvers that need them
  // ... implementation
}

// Similar updates for graphql_get_handler if needed
```

Update GraphQL mutations that use `set_session_cookie()` (likely in `src/graphql/mutations/create_session.rs`):
```rust
// Update to pass config parameters to set_session_cookie
```

Update `src/main.rs`:
```rust
#[tokio::main]
async fn main() {
  // ... existing setup ...

  // Read all configuration from environment
  let session_secret = get_secret_from_env("DP_AUTH_SECRET_KEY", 32)
    .expect("Failed to read session secret");
  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());
  let cookie_domain = env::var("DP_AUTH_COOKIE_DOMAIN")
    .unwrap_or_else(|_| ".api.dp-auth.localhost".to_string());
  let insecure_cookie = env::var("DP_AUTH_INSECURE_COOKIE").is_ok();
  let development_mode = env::var("APP_ENV").unwrap_or_default() == "development";

  // ... database setup ...

  let app = Router::new()
    .route("/", get(root_handler))
    .route("/health", get(health_handler))
    .route(
      "/graphql",
      get({
        let cookie_domain = cookie_domain.clone();
        move |state, headers, request| {
          graphql_get_handler(state, headers, request, cookie_domain, insecure_cookie, development_mode)
        }
      })
      .post({
        let cookie_domain = cookie_domain.clone();
        move |state, headers, request| {
          graphql_post_handler(state, headers, request, cookie_domain, insecure_cookie, development_mode)
        }
      })
      .layer(middleware::from_fn(create_session_middleware(session_secret))),
    )
    // ... rest remains the same
}
```

**Testing**: Run all existing tests, especially GraphQL integration tests, to ensure functionality is unchanged.

### Phase 4: Configuration Structure and start_server Function

**Goal**: Create ServerConfig struct and start_server() function to centralize configuration.

**Current State**: main.rs reads individual environment variables and passes them around

**Target State**: ServerConfig struct holds all configuration, start_server() function accepts it

**Files to Create/Modify**:
- `src/config.rs` - New file with ServerConfig struct
- `src/lib.rs` - Add start_server function and export config
- `src/main.rs` - Use new config structure

**Implementation Details**:

Create `src/config.rs`:
```rust
#[derive(Debug, Clone)]
pub struct ServerConfig {
  pub port: u16,
  pub database_url: String,
  pub session_secret: Vec<u8>, // 32-byte secret
  pub cookie_domain: String,
  pub insecure_cookie: bool,
  pub development_mode: bool,
}

impl ServerConfig {
  pub fn new(
    port: u16,
    database_url: String,
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
      database_url,
      session_secret,
      cookie_domain,
      insecure_cookie,
      development_mode,
    })
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
  InvalidSecretLength { actual: usize, expected: usize },
}

impl std::fmt::Display for ConfigError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ConfigError::InvalidSecretLength { actual, expected } => {
        write!(f, "Invalid secret length: got {actual} bytes, expected {expected}")
      }
    }
  }
}

impl std::error::Error for ConfigError {}
```

Update `src/lib.rs`:
```rust
pub mod config;
pub mod database;
pub mod graphql;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod queries;
pub mod services;
pub mod utils;

pub use config::{ServerConfig, ConfigError};

use axum::{middleware, routing::get, Router};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn start_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
  // Initialize tracing
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "dp_auth_service=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  // Initialize database connection
  let _database = database::Database::new(&config.database_url).await?;

  // Create schema
  let schema = graphql::schema::create_schema();

  // Build router with config
  let app = Router::new()
    .route("/", get(handlers::rest::root_handler))
    .route("/health", get(handlers::rest::health_handler))
    .route(
      "/graphql",
      get({
        let config = config.clone();
        move |state, headers, request| {
          handlers::graphql::graphql_get_handler(
            state, headers, request, 
            config.cookie_domain.clone(), 
            config.insecure_cookie, 
            config.development_mode
          )
        }
      })
      .post({
        let config = config.clone();
        move |state, headers, request| {
          handlers::graphql::graphql_post_handler(
            state, headers, request,
            config.cookie_domain.clone(),
            config.insecure_cookie,
            config.development_mode
          )
        }
      })
      .layer(middleware::from_fn(
        middleware::session::create_session_middleware(config.session_secret.clone())
      )),
    )
    .fallback(handlers::rest::not_found_handler)
    .layer(
      ServiceBuilder::new()
        .layer(middleware::from_fn(middleware::request_id::request_id_middleware))
        .layer(middleware::from_fn(middleware::logging::rest_logging_middleware))
        .layer(CorsLayer::permissive()),
    )
    .with_state(schema);

  // Bind to configured port
  let bind_address = format!("0.0.0.0:{}", config.port);
  let listener = TcpListener::bind(&bind_address).await?;

  println!("Server running on http://{bind_address}");

  // Create server with graceful shutdown
  let server = axum::serve(listener, app).with_graceful_shutdown(async {
    let signal_name = services::shutdown_service::ShutdownService::wait_for_shutdown_signal().await;
    services::shutdown_service::ShutdownService::log_shutdown_start(signal_name);
  });

  server.await?;
  Ok(())
}
```

Update `src/main.rs`:
```rust
use dp_auth_service::{start_server, ServerConfig};
use dp_auth_service::utils::get_secret_from_env::get_secret_from_env;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file
  dotenvy::dotenv().ok();

  // Read configuration from environment variables
  let port = env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string())
    .parse::<u16>()
    .expect("PORT must be a valid number");

  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  let session_secret = get_secret_from_env("DP_AUTH_SECRET_KEY", 32)
    .expect("Failed to read session secret from DP_AUTH_SECRET_KEY");

  let cookie_domain = env::var("DP_AUTH_COOKIE_DOMAIN")
    .unwrap_or_else(|_| ".api.dp-auth.localhost".to_string());

  let insecure_cookie = env::var("DP_AUTH_INSECURE_COOKIE").is_ok();

  let development_mode = env::var("APP_ENV").unwrap_or_default() == "development";

  let config = ServerConfig::new(
    port,
    database_url,
    session_secret,
    cookie_domain,
    insecure_cookie,
    development_mode,
  )?;

  start_server(config).await
}
```

**Testing**: Run all existing tests to ensure the new configuration system works correctly.

### Phase 5: Final Library Conversion

**Goal**: Remove main.rs, create local development script, update documentation.

**Current State**: Still has main.rs binary, Cargo.toml configured as binary

**Target State**: Pure library with separate development script

**Files to Create/Modify/Delete**:
- Delete `src/main.rs`
- Create `scripts/run_local_server.rs`
- Update `Cargo.toml`
- Update `README.md`

**Implementation Details**:

Create `scripts/run_local_server.rs`:
```rust
use dp_auth_service::{start_server, ServerConfig};
use dp_auth_service::utils::get_secret_from_env::get_secret_from_env;
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

  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  let session_secret = get_secret_from_env("DP_AUTH_SECRET_KEY", 32)
    .expect("DP_AUTH_SECRET_KEY environment variable is required");

  let cookie_domain = env::var("DP_AUTH_COOKIE_DOMAIN")
    .unwrap_or_else(|_| ".api.dp-auth.localhost".to_string());

  let insecure_cookie = env::var("DP_AUTH_INSECURE_COOKIE").is_ok();

  let development_mode = env::var("APP_ENV").unwrap_or_default() == "development";

  let config = ServerConfig::new(
    port,
    database_url,
    session_secret,
    cookie_domain,
    insecure_cookie,
    development_mode,
  )?;

  start_server(config).await
}
```

Update `Cargo.toml`:
```toml
[package]
name = "dp-auth-service"
version = "0.1.0"
edition = "2021"
# Remove: default-run = "dp-auth-service"

# Add local development script
[[bin]]
name = "run_local_server"
path = "scripts/run_local_server.rs"

# Keep existing migrate_and_dump binary
[[bin]]
name = "migrate_and_dump"
path = "config/scripts/migrate_and_dump.rs"
```

Update `README.md` to document library usage and local development.

**Testing**: Verify the library can be used by external crates and the local development script works.

## Testing Strategy for Each Phase

Each phase should be fully tested before moving to the next:

1. **Unit Tests**: Run `cargo test` to ensure all existing functionality works
2. **Integration Tests**: Run integration tests to verify end-to-end functionality
3. **Manual Testing**: Start the server and test key endpoints
4. **Regression Testing**: Ensure no existing functionality is broken

## Migration Path

This phased approach ensures:
- Each phase is small and manageable
- Functionality is preserved at each step
- Issues can be caught and fixed early
- The codebase remains in a working state throughout the process

### 1. Create Configuration Structure

Create a new `ServerConfig` struct in `src/config.rs` that contains all configuration parameters:

```rust
#[derive(Debug, Clone)]
pub struct ServerConfig {
  pub port: u16,
  pub database_url: String,
  pub session_secret: Vec<u8>, // 32-byte secret
  pub cookie_domain: String,
  pub insecure_cookie: bool,
  pub development_mode: bool,
}

impl ServerConfig {
  pub fn new(
    port: u16,
    database_url: String,
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
      database_url,
      session_secret,
      cookie_domain,
      insecure_cookie,
      development_mode,
    })
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
  InvalidSecretLength { actual: usize, expected: usize },
}
```

### 2. Refactor Session Middleware

Update `src/middleware/session.rs` to accept the secret as a parameter instead of reading from environment:

```rust
// Remove global SESSION_SECRET and related functions
// Update session_middleware to accept secret from config

pub fn create_session_middleware(secret: &'static [u8]) -> impl Fn(Request, Next) -> BoxFuture<'static, Response> + Clone {
  let secret = secret.to_vec();
  move |request: Request, next: Next| {
    let secret = secret.clone();
    Box::pin(async move {
      let session_context = extract_and_validate_session_sync(&request, &secret);
      let mut request = request;
      request.extensions_mut().insert(session_context);
      next.run(request).await
    })
  }
}

fn extract_and_validate_session_sync(request: &Request, secret: &[u8]) -> SessionContext {
  // Updated to use provided secret instead of global
}
```

### 3. Update Database Module

Modify `src/database/mod.rs` to accept database URL as parameter:

```rust
impl Database {
  pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
    // Use provided database_url instead of env::var
  }
}
```

### 4. Update GraphQL Handlers

Modify `src/handlers/graphql.rs` to use config instead of environment variables:

```rust
pub fn set_session_cookie(
  response_headers: &Arc<Mutex<HeaderMap>>, 
  token: &str,
  cookie_domain: &str,
  insecure_cookie: bool
) {
  // Use provided parameters instead of env::var
}

// Update other functions to accept config parameters
```

### 5. Create Main Library Function

Add a new `start_server()` function to `src/lib.rs`:

```rust
use crate::config::ServerConfig;
use axum::{middleware, routing::get, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

pub async fn start_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
  // Initialize tracing (keep existing setup)
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "dp_auth_service=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  // Initialize database connection
  let _database = Database::new(&config.database_url).await?;

  // Create schema
  let schema = create_schema();

  // Create session middleware with provided secret
  let session_middleware = create_session_middleware(&config.session_secret);

  // Build router with config-aware handlers
  let app = Router::new()
    .route("/", get(root_handler))
    .route("/health", get(health_handler))
    .route(
      "/graphql",
      get(move |state, headers, request| {
        graphql_get_handler(state, headers, request, config.clone())
      })
      .post(move |state, headers, request| {
        graphql_post_handler(state, headers, request, config.clone())
      })
      .layer(middleware::from_fn(session_middleware)),
    )
    .fallback(not_found_handler)
    .layer(
      ServiceBuilder::new()
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(rest_logging_middleware))
        .layer(CorsLayer::permissive()),
    )
    .with_state(schema);

  // Bind to configured port
  let bind_address = format!("0.0.0.0:{}", config.port);
  let listener = TcpListener::bind(&bind_address).await?;

  println!("Server running on http://{bind_address}");

  // Create server with graceful shutdown
  let server = axum::serve(listener, app).with_graceful_shutdown(async {
    let signal_name = ShutdownService::wait_for_shutdown_signal().await;
    ShutdownService::log_shutdown_start(signal_name);
  });

  // Run the server
  server.await?;
  Ok(())
}
```

### 6. Remove main.rs

Delete `src/main.rs` since the crate will no longer be a binary.

### 7. Update Cargo.toml

Remove the binary configuration:

```toml
[package]
name = "dp-auth-service"
version = "0.1.0"
edition = "2021"
# Remove: default-run = "dp-auth-service"

# Keep the migrate_and_dump binary
[[bin]]
name = "migrate_and_dump"
path = "config/scripts/migrate_and_dump.rs"
```

### 8. Create Standalone Test Script

Create `scripts/run_local_server.rs` for local testing:

```rust
use dp_auth_service::{start_server, ServerConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file for local development
  dotenvy::dotenv().ok();

  // Read configuration from environment variables (for local testing only)
  let port = env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string())
    .parse::<u16>()
    .expect("PORT must be a valid number");

  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  let session_secret = {
    use base64::{engine::general_purpose, Engine as _};
    let key_str = env::var("DP_AUTH_SECRET_KEY")
      .expect("DP_AUTH_SECRET_KEY environment variable is required");
    let key_bytes = general_purpose::STANDARD
      .decode(&key_str)
      .expect("DP_AUTH_SECRET_KEY must be valid base64");
    if key_bytes.len() < 32 {
      panic!("DP_AUTH_SECRET_KEY must decode to at least 32 bytes");
    }
    key_bytes[..32].to_vec()
  };

  let cookie_domain = env::var("DP_AUTH_COOKIE_DOMAIN")
    .unwrap_or_else(|_| ".api.dp-auth.localhost".to_string());

  let insecure_cookie = env::var("DP_AUTH_INSECURE_COOKIE").is_ok();

  let development_mode = env::var("APP_ENV").unwrap_or_default() == "development";

  let config = ServerConfig::new(
    port,
    database_url,
    session_secret,
    cookie_domain,
    insecure_cookie,
    development_mode,
  )?;

  start_server(config).await
}
```

### 9. Update migrate_and_dump Script

Update `config/scripts/migrate_and_dump.rs` to accept database URL as parameter:

```rust
use dp_auth_service::database::Database;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file
  dotenvy::dotenv().ok();

  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  println!("Starting database migration, seeding, and schema dump...");

  // Initialize database and run migrations
  let database = Database::new(&database_url).await?;
  // ... rest remains the same
}
```

### 10. Update Cargo.toml for Script

Add the local server script as a binary:

```toml
[[bin]]
name = "run_local_server"
path = "scripts/run_local_server.rs"
```

### 11. Update Documentation

Update `README.md` to reflect the new library usage:

```markdown
## Usage as Library

```rust
use dp_auth_service::{start_server, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let config = ServerConfig::new(
    3000, // port
    "sqlite:data/production.db".to_string(), // database_url
    your_32_byte_secret, // session_secret
    ".yourdomain.com".to_string(), // cookie_domain
    false, // insecure_cookie
    false, // development_mode
  )?;

  start_server(config).await
}
```

## Local Development

For local development and testing, use the provided script:

```bash
# Set environment variables in .env file
cargo run --bin run_local_server
```
```

## Files to be Created/Modified

### New Files:
- `src/config.rs` - Configuration structure
- `scripts/run_local_server.rs` - Local development script

### Modified Files:
- `src/lib.rs` - Add start_server function and export config
- `src/middleware/session.rs` - Remove global secret, accept as parameter
- `src/database/mod.rs` - Accept database URL as parameter
- `src/handlers/graphql.rs` - Use config instead of env vars
- `config/scripts/migrate_and_dump.rs` - Accept database URL as parameter
- `Cargo.toml` - Remove default-run, add new binary
- `README.md` - Update usage documentation

### Deleted Files:
- `src/main.rs` - No longer needed

## Testing Strategy

1. Update existing tests to use the new configuration approach
2. Create integration tests that verify the library can be used by external crates
3. Test the local development script to ensure it works with environment variables
4. Verify all existing functionality works with the new parameter-based approach

## Migration Path for Existing Users

Existing users can migrate by:
1. Creating a `ServerConfig` with their current environment variable values
2. Calling `dp_auth_service::start_server(config)` instead of running the binary
3. Using the provided `run_local_server` script for local development

This change maintains backward compatibility for local development while enabling library usage for production deployments.