# @dimensionalpocket/dps-auth-api

[![Rust Tests](https://github.com/dimensionalpocket/dps-auth-api/actions/workflows/test.yml/badge.svg)](https://github.com/dimensionalpocket/dps-auth-api/actions/workflows/test.yml) [![Docker Build Test](https://github.com/dimensionalpocket/dps-auth-api/actions/workflows/docker-build-test.yml/badge.svg)](https://github.com/dimensionalpocket/dps-auth-api/actions/workflows/docker-build-test.yml)

An opinionated authentication, user management, and session handling GraphQL API.

It is intended to be deployed as a microservice with a volume to house the SQLite databases.

## Features

- GraphQL API with authentication mutations and queries
- User management with role-based permissions
- Session token management with encrypted tokens
- Password hashing with Argon2
- SQLite database with migrations
- REST endpoints for health checks

## API Endpoints

- `GET /` - Root endpoint
- `GET /health` - Health check
- `GET /graphql` - GraphQL playground (development mode only)
- `POST /graphql` - GraphQL API

## GraphQL Operations

### Mutations
- `createUser(username: String!, password: String!)` - Create a new user
- `createSession(username: String!, password: String!)` - Sign in and create session

### Queries
- `getServerTimestamp` - Get current server timestamp
- `getCurrentSession` - Get current session information

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
dps-auth-api = { git = "https://github.com/dimensionalpocket/dps-auth-api", tag = "0.1.0" }
```

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
    config.set_auth_api_sqlite_main_pool_size(Some(10));
    
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
config.set_auth_api_sqlite_main_pool_size(Some(10));
let server = DpsAuthApi::new(config)?;
```

All configuration options except `auth_api_session_secret` have sensible defaults and are optional.

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

## Database Setup

### Using the Migration Binary

The library provides a `dps-auth-api-migrate` binary for database setup.

#### Building the Migration Binary

When you include this library as a dependency, you can build the migration binary:

    # Build the migration binary
    cargo build --bin dps-auth-api-migrate

    # Or build in release mode for production
    cargo build --release --bin dps-auth-api-migrate

The binary will be available at:
- Debug: `target/debug/dps-auth-api-migrate`
- Release: `target/release/dps-auth-api-migrate`

#### Running the Migration Binary

    # Basic usage with defaults (during development)
    cargo run --bin dps-auth-api-migrate

    # With custom SQLite file
    cargo run --bin dps-auth-api-migrate -- --sqlite-file ./my-auth.db

    # Skip seeds
    cargo run --bin dps-auth-api-migrate --skip-seeds

#### Production Deployment

For production deployments, build the binary in release mode and run it directly:

    # Build for production
    cargo build --release --bin dps-auth-api-migrate

    # Run the built binary
    ./target/release/dps-auth-api-migrate --sqlite-file ./production.db

    # Or with environment variables
    export DPS_AUTH_API_SQLITE_FILE="./production.db"
    export DPS_AUTH_API_MIGRATE_SKIP_SEEDS="false"
    ./target/release/dps-auth-api-migrate

#### Docker Deployment

If using Docker, include the binary in your Dockerfile:

    # Build stage
    FROM rust:1.88.0 as builder
    WORKDIR /app
    COPY . .
    RUN cargo build --release --bin your-app --bin dps-auth-api-migrate

    # Runtime stage
    FROM debian:bullseye-slim
    WORKDIR /app
    COPY --from=builder /app/target/release/your-app /app/your-app
    COPY --from=builder /app/target/release/dps-auth-api-migrate /app/dps-auth-api-migrate

    # Run migrations before starting your app
    CMD ["sh", "-c", "./dps-auth-api-migrate && ./your-app"]

### Configuration Methods

1. **Environment Variables**:
```bash
export DPS_AUTH_API_SQLITE_FILE="data/development.db"
export DPS_AUTH_API_MIGRATE_SKIP_SEEDS="false"
```

2. **Command-line Arguments** (see `--help` for full list)

## License

[MIT](./LICENSE)
