# Phase 2: Logging Implementation

**Date**: 2025-07-10  
**Phase**: 2 of 4  
**Goal**: Implement comprehensive logging for REST endpoints and GraphQL queries following industry standards

## Overview

This phase focuses on implementing structured logging for the dp-auth-service. We'll add logging capabilities to track REST endpoint requests and GraphQL query executions, providing essential observability for monitoring and debugging purposes.

## Industry Standards Research

### Recommended Logging Libraries for Rust
- **`tracing`** - Modern structured logging framework (recommended)
  - Provides spans and events for request tracing
  - Excellent integration with async code
  - Supports structured data and filtering
- **`tracing-subscriber`** - Subscriber implementations for tracing
- **`tracing-appender`** - File rotation and async writing
- **Alternative**: `log` + `env_logger` (simpler but less feature-rich)

### Logging Best Practices
- **Structured Logging**: Use key-value pairs instead of plain text
- **Log Levels**: DEBUG, INFO, WARN, ERROR appropriately
- **Request IDs**: Track requests across service boundaries
- **Performance Metrics**: Response times, status codes
- **Security**: Never log sensitive data (passwords, tokens)
- **Format**: JSON for production, human-readable for development

## Tasks Breakdown

### 1. Dependencies Setup
Add logging dependencies to `Cargo.toml`:
```toml
[dependencies]
# Existing dependencies...
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
uuid = { version = "1.0", features = ["v4"] }

[dev-dependencies]
# Existing dev dependencies...
tracing-test = "0.2"
```

### 2. Logging Infrastructure Setup
- Configure tracing subscriber in `main.rs`
- Set up environment-based log level configuration
- Configure JSON output for production, pretty output for development
- Add request ID generation middleware

### 3. REST Endpoint Logging
Implement logging for existing REST endpoints:
- **Log Fields for REST**:
  - Request method (GET, POST, etc.)
  - Request path
  - Response status code
  - Response time (milliseconds)
  - Request ID
  - User agent (optional)
  - IP address (optional)

**Endpoints to instrument**:
- `/` (root_handler)
- `/health` (health_handler)

### 4. GraphQL Query Logging
Implement logging for GraphQL operations:
- **Log Fields for GraphQL**:
  - Query/mutation name
  - Response time (milliseconds)
  - Request ID
  - Success/error status
  - **Note**: No query parameters logged for security

**Queries to instrument**:
- `getServerTimestamp` query

### 5. Middleware Implementation
Create logging middleware components:
- **Request ID Middleware**: Generate unique ID per request
- **REST Logging Middleware**: Log REST endpoint access
- **GraphQL Logging Integration**: Hook into async-graphql execution

### 6. Configuration Management
- Environment variable configuration for log levels
- Support for different output formats (JSON vs. pretty)
- Log file rotation configuration (for future file logging)

## Implementation Details

### Files to Create/Modify

#### 1. `src/middleware/mod.rs` (new)
```rust
pub mod logging;
pub mod request_id;
```

#### 2. `src/middleware/request_id.rs` (new)
```rust
use axum::{extract::Request, middleware::Next, response::Response};
use uuid::Uuid;

pub async fn request_id_middleware(
  mut request: Request,
  next: Next,
) -> Response {
  let request_id = Uuid::new_v4().to_string();
  request.extensions_mut().insert(RequestId(request_id));
  next.run(request).await
}

#[derive(Clone)]
pub struct RequestId(pub String);
```

#### 3. `src/middleware/logging.rs` (new)
```rust
use axum::{extract::Request, middleware::Next, response::Response};
use tracing::info;
use std::time::Instant;
use crate::middleware::request_id::RequestId;

pub async fn rest_logging_middleware(
  request: Request,
  next: Next,
) -> Response {
  let start = Instant::now();
  let method = request.method();
  let path = request.uri().path();
  let request_id = request.extensions().get::<RequestId>().cloned().unwrap_or(RequestId("unknown".to_string()));
  
  let response = next.run(request).await;
  
  let duration = start.elapsed();
  let status = response.status();
  
  info!(
    method = %method,
    path = path,
    status = %status,
    duration_ms = duration.as_millis(),
    request_id = %request_id.0,
    "REST Request"
  );
  
  response
}
```

#### 4. `src/handlers/rest.rs` (modify)
Add tracing spans to existing handlers:
```rust
use tracing::instrument;

#[instrument]
pub async fn root_handler() -> &'static str {
  "OK"
}

#[instrument]
pub async fn health_handler() -> &'static str {
  "OK"
}
```

#### 5. `src/handlers/graphql.rs` (modify)
Add GraphQL-specific logging:
```rust
use tracing::{info, error, instrument, Span};
use std::time::Instant;

#[instrument(skip(schema, req))]
pub async fn graphql_post_handler(
  State(schema): State<AppSchema>,
  req: GraphQLRequest,
) -> impl IntoResponse {
  let start = Instant::now();
  let request = req.into_inner();
  
  let response = schema.execute(request).await;
  let duration = start.elapsed();
  
  // Extract operation name from the response (more performant)
  let query_name = response
    .data
    .operation_name()
    .unwrap_or("unknown");
  
  if response.is_ok() {
    info!(
      query = query_name,
      duration_ms = duration.as_millis(),
      "GraphQL Request"
    );
  } else {
    error!(
      query = query_name,
      duration_ms = duration.as_millis(),
      "GraphQL Request Error"
    );
  }
  
  let graphql_response: GraphQLResponse = response.into();
  graphql_response
}
```

#### 6. `src/graphql/queries/get_server_timestamp.rs` (modify)
Add tracing instrument (no additional logging needed):
```rust
use tracing::instrument;

impl GetServerTimestampQuery {
  #[instrument]
  pub async fn get_server_timestamp(&self) -> i64 {
    ServerService::get_server_timestamp()
  }
}
```

#### 7. `src/main.rs` (modify)
Initialize tracing and add middleware:
```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tower::ServiceBuilder;
use axum::middleware;

#[tokio::main]
async fn main() {
  // Initialize tracing
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "dp_auth_service=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  // Load environment variables
  dotenvy::dotenv().ok();
  
  let schema = create_schema();
  
  let app = Router::new()
    .route("/", get(root_handler))
    .route("/health", get(health_handler))
    .route("/graphql", get(graphql_get_handler).post(graphql_post_handler))
    .layer(
      ServiceBuilder::new()
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(rest_logging_middleware))
        .layer(CorsLayer::permissive())
    )
    .with_state(schema);

  // ... rest of main function
}
```

### 7. Testing Strategy

#### Unit Tests for Middleware Components

**`tests/middleware/request_id_tests.rs`** (new):
```rust
use dp_auth_service::middleware::request_id::{request_id_middleware, RequestId};
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use uuid::Uuid;

#[tokio::test]
async fn test_request_id_middleware_generates_uuid() {
  let request = Request::builder().body(Body::empty()).unwrap();
  
  let response = request_id_middleware(request, |mut req| async move {
    // Verify request ID was added to extensions
    let request_id = req.extensions().get::<RequestId>().unwrap();
    assert!(Uuid::parse_str(&request_id.0).is_ok());
    
    Response::new(Body::empty())
  }).await;
  
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_request_id_middleware_unique_ids() {
  let mut ids = Vec::new();
  
  for _ in 0..10 {
    let request = Request::builder().body(Body::empty()).unwrap();
    
    request_id_middleware(request, |req| async move {
      let request_id = req.extensions().get::<RequestId>().unwrap();
      ids.push(request_id.0.clone());
      Response::new(Body::empty())
    }).await;
  }
  
  // Verify all IDs are unique
  let mut unique_ids = ids.clone();
  unique_ids.sort();
  unique_ids.dedup();
  assert_eq!(ids.len(), unique_ids.len());
}
```

**`tests/middleware/logging_tests.rs`** (new):
```rust
use dp_auth_service::middleware::logging::rest_logging_middleware;
use dp_auth_service::middleware::request_id::RequestId;
use axum::{body::Body, extract::Request, response::Response, http::StatusCode};
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn test_rest_logging_middleware_logs_request() {
  let mut request = Request::builder()
    .method("GET")
    .uri("/health")
    .body(Body::empty())
    .unwrap();
  
  // Add request ID to extensions
  request.extensions_mut().insert(RequestId("test-id-123".to_string()));
  
  let response = rest_logging_middleware(request, |_| async {
    Response::builder()
      .status(StatusCode::OK)
      .body(Body::empty())
      .unwrap()
  }).await;
  
  assert_eq!(response.status(), StatusCode::OK);
  
  // Verify log was captured
  assert!(logs_contain("REST Request"));
  assert!(logs_contain("GET"));
  assert!(logs_contain("/health"));
  assert!(logs_contain("200"));
  assert!(logs_contain("test-id-123"));
}
```

#### Integration Tests for Log Output

**`tests/integration/logging_integration_tests.rs`** (new):
```rust
use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;
use dp_auth_service::create_app;
use tracing_test::traced_test;
use serde_json::Value;

#[tokio::test]
#[traced_test]
async fn test_rest_endpoint_logging() {
  let app = create_app().await;
  
  let request = Request::builder()
    .method("GET")
    .uri("/health")
    .body(Body::empty())
    .unwrap();
  
  let response = app.oneshot(request).await.unwrap();
  
  assert_eq!(response.status(), StatusCode::OK);
  
  // Verify structured log output
  let logs = captured_logs();
  let log_entry: Value = serde_json::from_str(&logs[0]).unwrap();
  
  assert_eq!(log_entry["message"], "REST Request");
  assert_eq!(log_entry["method"], "GET");
  assert_eq!(log_entry["path"], "/health");
  assert_eq!(log_entry["status"], "200 OK");
  assert!(log_entry["duration_ms"].is_number());
  assert!(log_entry["request_id"].is_string());
}

#[tokio::test]
#[traced_test]
async fn test_graphql_query_logging() {
  let app = create_app().await;
  
  let graphql_query = r#"
    {
      "query": "query getServerTimestamp { getServerTimestamp }"
    }
  "#;
  
  let request = Request::builder()
    .method("POST")
    .uri("/graphql")
    .header("content-type", "application/json")
    .body(Body::from(graphql_query))
    .unwrap();
  
  let response = app.oneshot(request).await.unwrap();
  
  assert_eq!(response.status(), StatusCode::OK);
  
  // Verify GraphQL-specific logging
  let logs = captured_logs();
  let graphql_log: Value = logs.iter()
    .find(|log| log["message"] == "GraphQL Request")
    .unwrap();
  
  assert_eq!(graphql_log["query"], "getServerTimestamp");
  assert!(graphql_log["duration_ms"].is_number());
  assert!(graphql_log["request_id"].is_string());
}
```

#### Log Format and Level Tests

**`tests/integration/log_format_tests.rs`** (new):
```rust
use std::env;
use tracing_subscriber::fmt::format::Format;

#[tokio::test]
async fn test_json_log_format() {
  env::set_var("LOG_FORMAT", "json");
  
  // Initialize logging with JSON format
  let subscriber = create_test_subscriber_json();
  
  // Make request and verify JSON output
  let logs = capture_logs_with_subscriber(subscriber, || async {
    make_test_request().await;
  }).await;
  
  // Verify each log line is valid JSON
  for log_line in logs {
    let parsed: serde_json::Value = serde_json::from_str(&log_line)
      .expect("Log should be valid JSON");
    
    assert!(parsed["timestamp"].is_string());
    assert!(parsed["level"].is_string());
    assert!(parsed["message"].is_string());
  }
}

#[tokio::test]
async fn test_pretty_log_format() {
  env::set_var("LOG_FORMAT", "pretty");
  env::set_var("DP_AUTH_ENV", "development");
  
  let subscriber = create_test_subscriber_pretty();
  
  let logs = capture_logs_with_subscriber(subscriber, || async {
    make_test_request().await;
  }).await;
  
  // Verify human-readable format
  for log_line in logs {
    assert!(log_line.contains("INFO"));
    assert!(log_line.contains("dp_auth_service"));
    // Should NOT be JSON
    assert!(!log_line.starts_with("{"));
  }
}
```

#### Request ID Propagation Tests

**`tests/integration/request_id_propagation_tests.rs`** (new):
```rust
#[tokio::test]
async fn test_request_id_propagation_across_layers() {
  let app = create_app().await;
  
  let request = Request::builder()
    .method("POST")
    .uri("/graphql")
    .header("content-type", "application/json")
    .body(Body::from(r#"{"query": "{ getServerTimestamp }"}"#))
    .unwrap();
  
  let response = app.oneshot(request).await.unwrap();
  
  let logs = captured_logs();
  
  // Extract request IDs from all log entries for this request
  let request_ids: Vec<&str> = logs.iter()
    .filter_map(|log| log.get("request_id"))
    .filter_map(|id| id.as_str())
    .collect();
  
  // Verify all logs for this request have the same request ID
  assert!(!request_ids.is_empty());
  let first_id = request_ids[0];
  assert!(request_ids.iter().all(|&id| id == first_id));
  
  // Verify it's a valid UUID
  assert!(uuid::Uuid::parse_str(first_id).is_ok());
}
```


#### Error Handling Tests

**`tests/integration/error_logging_tests.rs`** (new):
```rust
#[tokio::test]
async fn test_graphql_error_logging() {
  let app = create_app().await;
  
  // Send invalid GraphQL query
  let invalid_query = r#"{"query": "invalid syntax here"}"#;
  
  let request = Request::builder()
    .method("POST")
    .uri("/graphql")
    .header("content-type", "application/json")
    .body(Body::from(invalid_query))
    .unwrap();
  
  let response = app.oneshot(request).await.unwrap();
  
  let logs = captured_logs();
  let error_log = logs.iter()
    .find(|log| log["level"] == "ERROR")
    .expect("Should have error log");
  
  assert_eq!(error_log["message"], "GraphQL Request Error");
  assert!(error_log["duration_ms"].is_number());
  assert!(error_log["request_id"].is_string());
}
```

#### Test Utilities

**`tests/common/mod.rs`** (new):
```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::{Arc, Mutex};

pub struct LogCapture {
  pub logs: Arc<Mutex<Vec<String>>>,
}

impl LogCapture {
  pub fn new() -> Self {
    Self {
      logs: Arc::new(Mutex::new(Vec::new())),
    }
  }
  
  pub fn captured_logs(&self) -> Vec<String> {
    self.logs.lock().unwrap().clone()
  }
}

pub fn create_test_subscriber() -> impl tracing::Subscriber {
  tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer().json())
    .with(tracing_subscriber::EnvFilter::new("dp_auth_service=debug"))
}
```

### 8. Documentation Updates
- Update API documentation with logging information
- Add environment variable documentation
- Document log format and fields

## Expected Log Output Examples

### REST Endpoint Log
```json
{
  "timestamp": "2025-07-10T15:33:42.123Z",
  "level": "INFO",
  "message": "REST Request",
  "method": "GET",
  "path": "/health",
  "status": "200 OK",
  "duration_ms": 1,
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### GraphQL Query Log (Success)
```json
{
  "timestamp": "2025-07-10T15:33:42.456Z",
  "level": "INFO", 
  "message": "GraphQL Request",
  "query": "getServerTimestamp",
  "duration_ms": 5,
  "request_id": "550e8400-e29b-41d4-a716-446655440001"
}
```

### GraphQL Query Log (Error)
```json
{
  "timestamp": "2025-07-10T15:33:42.789Z",
  "level": "ERROR", 
  "message": "GraphQL Request Error",
  "query": "getServerTimestamp",
  "duration_ms": 12,
  "request_id": "550e8400-e29b-41d4-a716-446655440002"
}
```

## Environment Variables

New environment variables to support:
- `RUST_LOG`: Log level configuration (e.g., "debug", "info", "warn", "error")
- `LOG_FORMAT`: Output format ("json" or "pretty")
- `DP_AUTH_ENV`: Environment name (affects log format defaults)

## Success Criteria

- [ ] All REST endpoints log request method, path, status, and response time
- [ ] GraphQL queries log query name and response time (no parameters)
- [ ] Request IDs are generated and propagated through logs
- [ ] Log output is structured and machine-readable
- [ ] Environment-based configuration works correctly
- [ ] All existing tests continue to pass
- [ ] New logging functionality has comprehensive test coverage
- [ ] Documentation is updated to reflect logging capabilities

## Future Considerations

- Log aggregation setup (ELK stack, etc.)
- Metrics collection integration
- Distributed tracing with OpenTelemetry
- Log sampling for high-traffic scenarios
- Security audit logging for authentication events
- Performance testing and optimization (separate phase)
- Load testing with logging overhead measurement