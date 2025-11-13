# dps-auth-api

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = DpsAuthApi::new()
        .session_secret(your_32_byte_secret)
        .build()?;

    server.start().await
}
```

### Complete Configuration Example

```rust
use dps_auth_api::DpsAuthApi;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = DpsAuthApi::new()
        .port(3000)
        .sqlite_file_path("data/production.db")
        .session_secret(your_32_byte_secret)
        .cookie_domain(".yourdomain.com")
        .insecure_cookie(false)
        .development_mode(false)
        .build()?;

    server.start().await
}
```

### With Logging Example

```rust
use dps_auth_api::DpsAuthApi;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dps_auth_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let server = DpsAuthApi::new()
        .session_secret(your_32_byte_secret)
        .build()?;

    server.start().await
}
```

## Configuration

The `DpsAuthApi` builder accepts the following configuration options:

- `port(u16)`: Server port (default: 3000)
- `sqlite_file_path(String)`: SQLite database file path (default: "data/development.db")
- `session_secret(Vec<u8>)`: 32-byte secret for session encryption (required)
- `cookie_domain(String)`: Domain for session cookies (default: ".api.dps-auth-api.localhost")
- `insecure_cookie(bool)`: Whether to use insecure cookies for development (default: false)
- `development_mode(bool)`: Enable development features like GraphQL playground (default: false)

All configuration options except `session_secret` have sensible defaults and are optional.

## Local Development

For local development, use the provided script:

```bash
# Set environment variables in .env file
cargo run --bin run_local_server
```

The `run_local_server` binary uses environment variables for configuration:
- `DPS_AUTH_SECRET_KEY`: Base64-encoded 32-byte secret (required)
- `DPS_AUTH_SQLITE_FILE`: SQLite database file path (optional, defaults to "data/development.db")
- `PORT`: Server port (optional, defaults to "3000")
- `DPS_AUTH_COOKIE_DOMAIN`: Cookie domain (optional, defaults to ".api.dps.localhost")
- `DPS_AUTH_INSECURE_COOKIE`: Set to enable insecure cookies (optional)
- `DPS_AUTH_ENV`: Set to "development" to enable development mode (optional)

**Note**: When using the library directly in your code, use the `DpsAuthApi` builder pattern instead of environment variables.

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

    # Using a configuration file
    cargo run --bin dps-auth-api-migrate --config ./my-config.toml

    # Skip seeds
    cargo run --bin dps-auth-api-migrate --skip-seeds

#### Production Deployment

For production deployments, build the binary in release mode and run it directly:

    # Build for production
    cargo build --release --bin dps-auth-api-migrate

    # Run the built binary
    ./target/release/dps-auth-api-migrate --sqlite-file ./production.db

    # Or with environment variables
    export DPS_AUTH_SQLITE_FILE="./production.db"
    export DPS_AUTH_MIGRATE_SKIP_SEEDS="false"
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

1. **Configuration File** (`dps-auth-api-migrate.toml`):
```toml
[database]
sqlite_file = "data/development.db"

[options]
skip_seeds = false
```

2. **Environment Variables**:
```bash
export DPS_AUTH_SQLITE_FILE="data/development.db"
export DPS_AUTH_MIGRATE_SKIP_SEEDS="false"
```

3. **Command-line Arguments** (see `--help` for full list)

## License

[MIT](./LICENSE)
