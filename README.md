# dp-auth-service

[![Rust Tests](https://github.com/dimensionalpocket/auth-service-rust/actions/workflows/test.yml/badge.svg)](https://github.com/dimensionalpocket/auth-service-rust/actions/workflows/test.yml) [![Docker Build Test](https://github.com/dimensionalpocket/auth-service-rust/actions/workflows/docker-build-test.yml/badge.svg)](https://github.com/dimensionalpocket/auth-service-rust/actions/workflows/docker-build-test.yml)

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
dp-auth-service = { git = "https://github.com/dimensionalpocket/auth-service-rust", tag = "0.1.0" }
```

## Usage

### Minimal Example (with defaults)

```rust
use dp_auth_service::DpAuthServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = DpAuthServer::new()
        .session_secret(your_32_byte_secret)
        .build()?;

    server.start().await
}
```

### Complete Configuration Example

```rust
use dp_auth_service::DpAuthServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = DpAuthServer::new()
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
use dp_auth_service::DpAuthServer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dp_auth_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let server = DpAuthServer::new()
        .session_secret(your_32_byte_secret)
        .build()?;

    server.start().await
}
```

## Configuration

The `DpAuthServer` builder accepts the following configuration options:

- `port(u16)`: Server port (default: 3000)
- `sqlite_file_path(String)`: SQLite database file path (default: "data/development.db")
- `session_secret(Vec<u8>)`: 32-byte secret for session encryption (required)
- `cookie_domain(String)`: Domain for session cookies (default: ".api.dp-auth.localhost")
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
- `DP_AUTH_SECRET_KEY`: Base64-encoded 32-byte secret (required)
- `DP_AUTH_SQLITE_FILE`: SQLite database file path (optional, defaults to "data/development.db")
- `PORT`: Server port (optional, defaults to "3000")
- `DP_AUTH_COOKIE_DOMAIN`: Cookie domain (optional, defaults to ".api.dp-auth.localhost")
- `DP_AUTH_INSECURE_COOKIE`: Set to enable insecure cookies (optional)
- `DP_AUTH_ENV`: Set to "development" to enable development mode (optional)

**Note**: When using the library directly in your code, use the `DpAuthServer` builder pattern instead of environment variables.

## Database Setup

### Using the Migration Binary

The library provides a `dp-auth-migrate` binary for database setup.

#### Building the Migration Binary

When you include this library as a dependency, you can build the migration binary:

    # Build the migration binary
    cargo build --bin dp-auth-migrate

    # Or build in release mode for production
    cargo build --release --bin dp-auth-migrate

The binary will be available at:
- Debug: `target/debug/dp-auth-migrate`
- Release: `target/release/dp-auth-migrate`

#### Running the Migration Binary

    # Basic usage with defaults (during development)
    cargo run --bin dp-auth-migrate

    # With custom SQLite file
    cargo run --bin dp-auth-migrate -- --sqlite-file ./my-auth.db

    # Using a configuration file
    cargo run --bin dp-auth-migrate --config ./my-config.toml

    # Skip seeds
    cargo run --bin dp-auth-migrate --skip-seeds

#### Production Deployment

For production deployments, build the binary in release mode and run it directly:

    # Build for production
    cargo build --release --bin dp-auth-migrate

    # Run the built binary
    ./target/release/dp-auth-migrate --sqlite-file ./production.db

    # Or with environment variables
    export DP_AUTH_SQLITE_FILE="./production.db"
    export DP_AUTH_MIGRATE_SKIP_SEEDS="false"
    ./target/release/dp-auth-migrate

#### Docker Deployment

If using Docker, include the binary in your Dockerfile:

    # Build stage
    FROM rust:1.88.0 as builder
    WORKDIR /app
    COPY . .
    RUN cargo build --release --bin your-app --bin dp-auth-migrate

    # Runtime stage
    FROM debian:bullseye-slim
    WORKDIR /app
    COPY --from=builder /app/target/release/your-app /app/your-app
    COPY --from=builder /app/target/release/dp-auth-migrate /app/dp-auth-migrate

    # Run migrations before starting your app
    CMD ["sh", "-c", "./dp-auth-migrate && ./your-app"]

### Configuration Methods

1. **Configuration File** (`dp-auth-migrate.toml`):
```toml
[database]
sqlite_file = "data/development.db"

[options]
skip_seeds = false
```

2. **Environment Variables**:
```bash
export DP_AUTH_SQLITE_FILE="data/development.db"
export DP_AUTH_MIGRATE_SKIP_SEEDS="false"
```

3. **Command-line Arguments** (see `--help` for full list)

## License

[MIT](./LICENSE)
