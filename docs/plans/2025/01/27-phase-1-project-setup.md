# Phase 1: Project Setup with REST Endpoints and getServerTimestamp Query

**Date**: 2025-01-27  
**Phase**: 1 of 3  
**Goal**: Initialize Rust project with basic REST endpoints and GraphQL query functionality

## Overview

This phase establishes the foundation of the dp-auth-service by setting up the Rust project structure, configuring essential dependencies, implementing basic REST endpoints, and creating the first GraphQL query with its associated service layer.

## Tasks Breakdown

### 1. Project Initialization
- Initialize new Rust project with name `dp-auth-service`
- Configure Cargo.toml with required dependencies
- Set up proper project structure following the design patterns

### 2. Dependency Configuration
- `async-graphql` - GraphQL server implementation
- `axum` - Web framework for REST endpoints
- `sqlx` - Database toolkit (prepared for future phases)
- `tokio` - Async runtime
- `serde` - Serialization/deserialization
- `dotenvy` - Environment variable loading
- Development dependencies for testing

### 3. REST Endpoints Implementation
- `/` endpoint returning simple "OK" message
- `/health` endpoint returning simple "OK" message
- Proper HTTP status codes and JSON responses

### 4. GraphQL Infrastructure
- GraphQL schema setup with async-graphql
- `/graphql` endpoint configuration
- Query resolver structure

### 5. getServerTimestamp Implementation
- `ServerService` with static `get_server_timestamp` method
- GraphQL query resolver calling the service
- Return current timestamp in milliseconds since epoch

### 6. Testing Strategy
- Unit tests for `ServerService::get_server_timestamp`
- Integration tests for GraphQL query
- REST endpoint testing
- Test coverage verification

### 7. Documentation
- API endpoint documentation
- GraphQL query documentation
- Code documentation with Rust doc comments

## File Structure

```
dp-auth-service/
├── Cargo.toml
├── .env
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── graphql/
│   │   ├── mod.rs
│   │   ├── queries/
│   │   │   ├── mod.rs
│   │   │   └── get_server_timestamp.rs
│   │   ├── query.rs
│   │   └── schema.rs
│   ├── services/
│   │   ├── mod.rs
│   │   └── server_service.rs
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── rest.rs
│   │   └── graphql.rs
│   └── routes/
│       └── mod.rs
├── tests/
│   ├── integration_tests.rs
│   └── graphql_tests.rs
└── docs/
    └── api.md
```

## Files to be Created/Modified

### 0. `.env`
```
# Development environment variables
PORT=3000
```

### 1. `Cargo.toml`
```toml
[package]
name = "dp-auth-service"
version = "0.1.0"
edition = "2021"

[dependencies]
async-graphql = "7.0"
async-graphql-axum = "7.0"
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }
dotenvy = "0.15"

[dev-dependencies]
tower-test = "0.4"
```

### 2. `src/main.rs`
```rust
use axum::{
  routing::{get, post},
  Router,
};
use dp_auth_service::{
  handlers::{graphql::graphql_handler, rest::{health_handler, root_handler}},
  graphql::schema::create_schema,
};
use tower_http::cors::CorsLayer;
use std::env;

#[tokio::main]
async fn main() {
  // Load environment variables from .env file
  dotenvy::dotenv().ok();
  
  let schema = create_schema();
  
  let app = Router::new()
    .route("/", get(root_handler))
    .route("/health", get(health_handler))
    .route("/graphql", post(graphql_handler))
    .layer(CorsLayer::permissive())
    .with_state(schema);

  // Get port from environment variable, default to 3000
  let port = env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string());
  let bind_address = format!("0.0.0.0:{}", port);

  let listener = tokio::net::TcpListener::bind(&bind_address)
    .await
    .unwrap();
    
  println!("Server running on http://{}", bind_address);
  axum::serve(listener, app).await.unwrap();
}
```

### 3. `src/services/server_service.rs`
```rust
use std::time::{SystemTime, UNIX_EPOCH};

/// Service for server-related operations
pub struct ServerService;

impl ServerService {
  /// Returns the current server timestamp in milliseconds since Unix epoch
  /// 
  /// # Returns
  /// 
  /// Current timestamp as u64 representing milliseconds since epoch
  /// 
  /// # Examples
  /// 
  /// ```
  /// use dp_auth_service::services::ServerService;
  /// 
  /// let timestamp = ServerService::get_server_timestamp();
  /// assert!(timestamp > 0);
  /// ```
  pub fn get_server_timestamp() -> u64 {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("Time went backwards")
      .as_millis() as u64
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::thread;
  use std::time::Duration;

  #[test]
  fn test_get_server_timestamp_returns_positive_value() {
    let timestamp = ServerService::get_server_timestamp();
    assert!(timestamp > 0);
  }

  #[test]
  fn test_get_server_timestamp_increases_over_time() {
    let timestamp1 = ServerService::get_server_timestamp();
    thread::sleep(Duration::from_millis(1));
    let timestamp2 = ServerService::get_server_timestamp();
    assert!(timestamp2 > timestamp1);
  }

  #[test]
  fn test_get_server_timestamp_reasonable_range() {
    let timestamp = ServerService::get_server_timestamp();
    // Should be after 2020-01-01 and before 2030-01-01
    let min_timestamp = 1577836800000u64; // 2020-01-01 in ms
    let max_timestamp = 1893456000000u64; // 2030-01-01 in ms
    assert!(timestamp >= min_timestamp);
    assert!(timestamp <= max_timestamp);
  }
}
```

### 4. `src/graphql/queries/get_server_timestamp.rs`
```rust
use async_graphql::{Object, Result};
use crate::services::ServerService;

/// Server timestamp query resolver
pub struct GetServerTimestampQuery;

#[Object]
impl GetServerTimestampQuery {
  /// Returns the current server timestamp in milliseconds since Unix epoch
  async fn get_server_timestamp(&self) -> Result<String> {
    let timestamp = ServerService::get_server_timestamp();
    Ok(timestamp.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_get_server_timestamp_calls_service() {
    let query = GetServerTimestampQuery;
    let result = query.get_server_timestamp().await;
    
    assert!(result.is_ok());
    let timestamp_str = result.unwrap();
    let timestamp: u64 = timestamp_str.parse().unwrap();
    assert!(timestamp > 0);
  }

  #[tokio::test]
  async fn test_get_server_timestamp_returns_string() {
    let query = GetServerTimestampQuery;
    let result = query.get_server_timestamp().await;
    
    assert!(result.is_ok());
    let timestamp_str = result.unwrap();
    // Should be parseable as u64
    assert!(timestamp_str.parse::<u64>().is_ok());
  }
}
```

### 5. `src/graphql/queries/mod.rs`
```rust
pub mod get_server_timestamp;

pub use get_server_timestamp::GetServerTimestampQuery;
```

### 6. `src/graphql/query.rs`
```rust
use async_graphql::{Object, Result, MergedObject};
use crate::graphql::queries::GetServerTimestampQuery;

/// Root query object for GraphQL API
/// 
/// This struct combines all individual query resolvers using MergedObject
#[derive(MergedObject, Default)]
pub struct Query(GetServerTimestampQuery);

impl Query {
  pub fn new() -> Self {
    Self(GetServerTimestampQuery)
  }
}
```

### 7. `src/handlers/rest.rs`
```rust
/// Handler for the root endpoint "/"
/// 
/// Returns a simple "OK" string with 200 status code
pub async fn root_handler() -> &'static str {
  "OK"
}

/// Handler for the health check endpoint "/health"
/// 
/// Returns a simple "OK" string with 200 status code
pub async fn health_handler() -> &'static str {
  "OK"
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_root_handler_returns_ok() {
    let result = root_handler().await;
    assert_eq!(result, "OK");
  }

  #[tokio::test]
  async fn test_health_handler_returns_ok() {
    let result = health_handler().await;
    assert_eq!(result, "OK");
  }
}
```

### 8. `src/graphql/schema.rs`
```rust
use async_graphql::{Schema, EmptyMutation, EmptySubscription};
use crate::graphql::query::Query;

/// GraphQL schema type definition
pub type AppSchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// Creates and returns the GraphQL schema
/// 
/// This function initializes the GraphQL schema with the Query resolver.
/// Mutations and subscriptions are empty for Phase 1.
pub fn create_schema() -> AppSchema {
  Schema::build(Query::new(), EmptyMutation, EmptySubscription)
    .finish()
}
```

### 9. `src/handlers/graphql.rs`
```rust
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
  extract::State,
  response::IntoResponse,
};
use crate::graphql::schema::AppSchema;

/// GraphQL endpoint handler
/// 
/// Processes GraphQL requests and returns responses
pub async fn graphql_handler(
  State(schema): State<AppSchema>,
  req: GraphQLRequest,
) -> impl IntoResponse {
  let response: GraphQLResponse = schema.execute(req.into_inner()).await.into();
  response
}
```

## Testing Strategy

### Unit Tests
- `ServerService::get_server_timestamp()` - Test return value, time progression, reasonable range
- REST handlers - Test return values and JSON structure
- GraphQL Query - Test service method calls with correct parameters

### Integration Tests
- Full HTTP requests to REST endpoints
- GraphQL query execution through HTTP
- End-to-end timestamp retrieval

### Test Commands
```bash
# Run all tests
mise exec -- cargo test

# Run tests with coverage
mise exec -- cargo test --verbose

# Run specific test module
mise exec -- cargo test services::server_service::tests
```

## API Documentation

### REST Endpoints

#### GET /
Returns service status information.

**Response:**
```
OK
```

#### GET /health
Returns health check status.

**Response:**
```
OK
```

### GraphQL Endpoint

#### POST /graphql

**Query: getServerTimestamp**
```graphql
query {
  getServerTimestamp
}
```

**Response:**
```json
{
  "data": {
    "getServerTimestamp": "1706356800000"
  }
}
```

## Success Criteria

1. ✅ Rust project initializes and compiles successfully
2. ✅ All dependencies are properly configured
3. ✅ REST endpoints return correct responses
4. ✅ GraphQL endpoint accepts and processes queries
5. ✅ `getServerTimestamp` query returns valid timestamp
6. ✅ `ServerService` follows static method pattern
7. ✅ All tests pass with good coverage
8. ✅ Code follows 2-space indentation standard
9. ✅ Documentation is complete and accurate

## Next Steps

After Phase 1 completion:
- Phase 2: Implement PasswordService for user password management
- Phase 3: Configure SQLite database with migrations

## Notes

- Using 2-space indentation as specified in project configuration
- Following static service pattern - no instantiation required
- GraphQL queries only call service methods and handle responses
- All service methods will have comprehensive unit tests
- Project structure supports future database integration