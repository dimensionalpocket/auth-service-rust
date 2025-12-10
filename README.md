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
- `GET {api_path}/graphql` - GraphQL playground (development mode only)
- `POST {api_path}/graphql` - GraphQL API

## GraphQL Operations

#### Queries

| Operation | Description |
|-----------|-------------|
| `getServerTimestamp` | Get current server timestamp. Returns timestamp (String). No authentication required. |
| `authMe` | Get current authenticated user profile. Returns userId (Int), uuid (String), username (String), roleId (Int), roleName (String), createdTs (Int), updatedTs (Int), sessionIat (Int), sessionExp (Int). Requires valid session cookie. |
| `site` | Get complete site details by ID (admin only). Returns id (Int!), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), createdTs (Int), updatedTs (Int). Requires can_view_site_details permission. |
| `sites` | List all sites in database. Returns array of [id (Int), slug (String), subdomain (String), port (Int), protocol (String)]. No authentication required. |
| `user` | Get complete user details by ID (admin only). Returns id (Int!), uuid (String), username (String), roleId (Int), roleName (String), createdTs (Int), updatedTs (Int). Requires can_view_user_details permission. |
| `users` | List all users with role information (admin only). Returns array of [id (Int), uuid (String), username (String), roleName (String), createdTs (Int), updatedTs (Int)]. Requires can_list_users permission. |
| `rolePermissions` | List all available role permissions. Returns array of permission strings. Requires can_manage_roles permission. |

#### Mutations

| Operation | Description |
|-----------|-------------|
| `authRegister` | Register new user account. Input: username (String!), password (String!), passwordConfirmation (String!). Returns: userId (Int), uuid (String), username (String), roleId (Int), createdTs (Int), updatedTs (Int), message (String). No authentication required. |
| `authLogin` | Authenticate user and create session. Input: username (String!), password (String!). Returns: token (String), userId (Int), username (String), message (String). No authentication required (sets cookie). |
| `authLogout` | Logout user by clearing session cookie. No input required. Returns: message (String). No authentication required. |
| `authChangePassword` | Change password for authenticated user. Input: currentPassword (String!), newPassword (String!), newPasswordConfirmation (String!). Returns: message (String). Requires valid session cookie. |
| `addSite` | Add new site to database. Input: slug (String!), subdomain (String), port (Int), protocol (String), metadataJson (String). Returns: id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), createdTs (Int), updatedTs (Int). Requires can_create_site permission. |
| `updateSite` | Update existing site. Input: id (Int!), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String). Returns: id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), createdTs (Int), updatedTs (Int). Requires can_update_site permission. |
| `removeSite` | Remove existing site. Input: siteId (Int!). Returns: id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), createdTs (Int), updatedTs (Int). Requires can_delete_site permission. |
| `deleteUser` | Delete existing user. Input: id (Int!). Returns: success (Boolean). Requires can_delete_user permission. |
| `updateUser` | Update existing user. Input: id (Int!), username (String), roleId (Int), password (String), passwordConfirmation (String), metadataJson (String). Returns: id (Int), uuid (String), username (String), roleId (Int), createdTs (Int), updatedTs (Int). Requires can_edit_user permission. |

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
dps-auth-api = { git = "https://github.com/dimensionalpocket/dps-auth-api", tag = "0.1.0" }
```

## Usage

### Quickstart (minimal example)

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

### Full example (complete configuration)

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


## Configuration

The `DpsAuthApi` uses `DpsConfig` from the `dps-config` crate for configuration management.

### Environment Variables

All configuration is loaded from environment variables via `DpsConfig::new()`:

- `DPS_DOMAIN`: Base domain (default: "dps.localhost")
- `DPS_API_PATH`: API path, without leading slash (default: "api", meaning DpsAuthApi serves at "/api/graphql")
- `DPS_DEVELOPMENT_MODE`: Set to "Y" to enable development features like GraphQL playground (default: not defined)
- `DPS_AUTH_API_PORT`: Server port (default: 3000 if not set)
- `DPS_AUTH_API_SQLITE_MAIN_FILE_PATH`: SQLite database file path (default: "data/main-development.db")
- `DPS_AUTH_API_SQLITE_MAIN_POOL_SIZE`: Database connection pool size (default: 1)
- `DPS_AUTH_API_SESSION_SECRET`: 32-byte secret for session encryption (required)
- `DPS_AUTH_API_INSECURE_COOKIE`: Set to "Y" to enable insecure cookies (default: false)

The cookie domain is automatically derived as `.{domain}` (e.g., ".dps.localhost").
The cookie path is set to the `api_path` configuration with a leading slash (e.g., "/api").

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

For local development, set the environment variables (see the "Environment Variables" section) and run the provided binary:

```bash
# Set environment variables in .env file, then:
cargo run --bin run_local_server
```

**Note**: When using the library directly in your code, use the `DpsAuthApi::new(config)` pattern instead.

## Database Setup

The library provides a `dps-auth-api-migrate` binary for database setup and rollback.

### Forward Migrations

- Build (debug): `cargo build --bin dps-auth-api-migrate`
- Build (release): `cargo build --release --bin dps-auth-api-migrate`
- Run migrations (development): `cargo run --bin dps-auth-api-migrate`
- Run with custom SQLite file: `cargo run --bin dps-auth-api-migrate -- --sqlite-file ./my-auth.db`
- Production: `./target/release/dps-auth-api-migrate --sqlite-file ./production.db`

### Migration Rollback

Rollback the last migration:
```bash
cargo run --bin dps-auth-api-migrate -- --revert
```

Rollback multiple migrations:
```bash
cargo run --bin dps-auth-api-migrate -- --revert 3
```

Rollback in production:
```bash
./target/release/dps-auth-api-migrate --sqlite-file ./production.db --revert
```

**Important**: Rollback will automatically regenerate the schema dump after reverting migrations.

Docker note:
- Build the migration binary in your builder stage and run it before starting your app in the runtime stage. See the repo Dockerfile or docs for an example.

### Configuration Methods

1. **Environment Variables**:
```bash
export DPS_AUTH_API_SQLITE_FILE="data/development.db"
export DPS_AUTH_API_MIGRATE_SKIP_SEEDS="false"
```

2. **Command-line Arguments**:
   - `--sqlite-file <PATH>` - SQLite database file path
   - `--skip-seeds` - Skip running seed files
   - `--revert [N]` - Revert last N migrations (default: 1)
   - `--help` - Show all available options

## License

[MIT](./LICENSE)
