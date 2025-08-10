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
- `DATABASE_URL`: SQLite database path (optional, defaults to "sqlite:data/development.db")
- `PORT`: Server port (optional, defaults to "3000")
- `DP_AUTH_COOKIE_DOMAIN`: Cookie domain (optional, defaults to ".api.dp-auth.localhost")
- `DP_AUTH_INSECURE_COOKIE`: Set to enable insecure cookies (optional)
- `APP_ENV`: Set to "development" to enable development mode (optional)

## Database Setup

Run migrations and seed data:

```bash
cargo run --bin migrate_and_dump
```

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
