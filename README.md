# dp-auth-service

A Rust library providing GraphQL-based authentication and user management services.

## Features

- GraphQL API with authentication mutations and queries
- User management with role-based permissions
- Session token management with encrypted tokens
- Password hashing with Argon2
- SQLite database with migrations
- REST endpoints for health checks

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
dp-auth-service = { git = "https://github.com/dimensionalpocket/auth-service-rust", tag = "0.1.0" }
```

## Usage

```rust
use dp_auth_service::{start_server, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::new(
        3000, // port
        "sqlite:data/production.db".to_string(), // database_url
        your_32_byte_secret, // session_secret (Vec<u8>)
        ".yourdomain.com".to_string(), // cookie_domain
        false, // insecure_cookie
        false, // development_mode
    )?;

    start_server(config).await
}
```

## Configuration

The `ServerConfig` struct accepts the following parameters:

- `port`: Server port (u16)
- `database_url`: SQLite database file path (String)
- `session_secret`: 32-byte secret for session encryption (Vec<u8>)
- `cookie_domain`: Domain for session cookies (String)
- `insecure_cookie`: Whether to use insecure cookies for development (bool)
- `development_mode`: Enable development features like GraphQL playground (bool)

## Local Development

For local development, use the provided script:

```bash
# Set environment variables in .env file
cargo run --bin run_local_server
```

Required environment variables:
- `DP_AUTH_SECRET_KEY`: Base64-encoded 32-byte secret
- `DP_AUTH_SQLITE_FILE`: SQLite database path (optional, defaults to "sqlite:data/development.db")
- `PORT`: Server port (optional, defaults to "3000")
- `DP_AUTH_COOKIE_DOMAIN`: Cookie domain (optional, defaults to ".api.dp-auth.localhost")
- `DP_AUTH_INSECURE_COOKIE`: Set to enable insecure cookies (optional)
- `APP_ENV`: Set to "development" to enable development mode (optional)

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
