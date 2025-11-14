# Phase 13: Handle 404s in the REST API

**Date**: 2025-07-12@09:48

## Overview

Implement a 404 handler for the REST API to handle requests to non-existent endpoints. This will provide proper error responses and logging for invalid routes.

## Requirements

From the README Phase 13:
- Implement a 404 handler for the REST API
- Requests to non-existent endpoints should return a 404 status code with a plain "NOT FOUND" message
- Requests should be logged with the request method, path, querystring, and content length (if present); log level should be `info`
- Integration tests for the 404 handler

## Current State Analysis

### Existing REST API Structure
- Current routes: `/` and `/health` (both GET)
- GraphQL endpoint: `/graphql` (GET and POST)
- Logging middleware: `rest_logging_middleware` in `src/middleware/logging.rs`
- Current logging only covers `/` and `/health` paths

### Router Configuration
In `src/main.rs`, the app is built with:
```rust
let app = Router::new()
  .route("/", get(root_handler))
  .route("/health", get(health_handler))
  .route("/graphql", ...)
  .layer(...)
```

## Implementation Plan

### 1. Create 404 Handler Function

**File**: `src/handlers/rest.rs`

Add a new handler function:
```rust
use axum::{http::StatusCode, response::Response, body::Body, extract::Request};

/// Handler for 404 Not Found responses
/// 
/// Returns a plain "NOT FOUND" message with 404 status code
pub async fn not_found_handler() -> (StatusCode, &'static str) {
  (StatusCode::NOT_FOUND, "NOT FOUND")
}
```

### 2. Update Logging Middleware

**File**: `src/middleware/logging.rs`

Extend the logging middleware to handle 404 requests:

```rust
pub async fn rest_logging_middleware(request: Request, next: Next) -> Response {
  let path = request.uri().path();
  let query = request.uri().query().unwrap_or("");
  let method = request.method().clone();
  
  // Calculate content length if present
  let content_length = if let Some(content_length_header) = request.headers().get("content-length") {
    content_length_header.to_str().unwrap_or("0").parse::<usize>().unwrap_or(0)
  } else {
    0
  };
  
  let start = Instant::now();
  let response = next.run(request).await;
  let duration = start.elapsed();
  let status = response.status();
  
  // Log for REST endpoints (existing routes + 404s)
  let should_log = path == "/" || path == "/health" || status == StatusCode::NOT_FOUND;
  
  if should_log {
    let query_part = if query.is_empty() { 
      String::new() 
    } else { 
      format!("?{}", query) 
    };
    
    let content_length_info = if content_length > 0 {
      format!(", content_length = {}", content_length)
    } else {
      String::new()
    };
    
    info!(
      method = %method,
      path = %format!("{}{}", path, query_part),
      status = %status,
      duration_ms = duration.as_millis(){},
      "REST Request"
    );
  }
  
  response
}
```

### 3. Add Fallback Route to Router

**File**: `src/main.rs`

Update the router to include a fallback handler:

```rust
let app = Router::new()
  .route("/", get(root_handler))
  .route("/health", get(health_handler))
  .route("/graphql", ...)
  .fallback(not_found_handler) // Add this line
  .layer(...)
```

Import the new handler:
```rust
use dp_auth_service::{
  // ... existing imports
  handlers::{
    graphql::{graphql_get_handler, graphql_post_handler},
    rest::{health_handler, root_handler, not_found_handler}, // Add not_found_handler
  },
  // ... rest of imports
};
```

### 4. Unit Tests

**File**: `src/handlers/rest.rs`

Add unit test for the 404 handler:
```rust
#[cfg(test)]
mod tests {
  use super::*;
  
  // ... existing tests
  
  #[tokio::test]
  async fn test_not_found_handler_returns_404() {
    let (status, message) = not_found_handler().await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(message, "NOT FOUND");
  }
}
```

### 5. Integration Tests

**File**: `tests/integration_tests.rs`

Add integration tests for 404 handling:
```rust
#[tokio::test]
async fn test_404_handler_returns_not_found() {
  let app = create_app();
  
  let request = Request::builder()
    .uri("/nonexistent")
    .body(Body::empty())
    .unwrap();
    
  let response = app.oneshot(request).await.unwrap();
  
  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  
  let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  assert_eq!(body_str, "NOT FOUND");
}

#[tokio::test]
async fn test_404_handler_with_query_string() {
  let app = create_app();
  
  let request = Request::builder()
    .uri("/nonexistent?param=value")
    .body(Body::empty())
    .unwrap();
    
  let response = app.oneshot(request).await.unwrap();
  
  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  
  let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  assert_eq!(body_str, "NOT FOUND");
}

#[tokio::test]
async fn test_404_handler_with_post_method() {
  let app = create_app();
  
  let request = Request::builder()
    .method("POST")
    .uri("/nonexistent")
    .header("content-type", "application/json")
    .body(Body::from(r#"{"test": "data"}"#))
    .unwrap();
    
  let response = app.oneshot(request).await.unwrap();
  
  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  
  let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body_str = String::from_utf8(body.to_vec()).unwrap();
  assert_eq!(body_str, "NOT FOUND");
}
```

## Files to be Created/Modified

### Modified Files:
1. **`src/handlers/rest.rs`**
   - Add `not_found_handler()` function
   - Add unit test for the handler

2. **`src/middleware/logging.rs`**
   - Update `rest_logging_middleware()` to log 404 requests
   - Include method, path, querystring, and content length in logs

3. **`src/main.rs`**
   - Add `.fallback(not_found_handler)` to the router
   - Import `not_found_handler` from handlers

4. **`tests/integration_tests.rs`**
   - Add integration tests for 404 scenarios
   - Test different HTTP methods and query strings

### No New Files Required

## Implementation Notes

### Logging Behavior
- 404 requests will be logged at `info` level as specified
- Log format will include: method, path (with querystring), status, duration, and content length if present
- Content length calculation uses the `content-length` header when available

### Router Fallback
- Axum's `.fallback()` method catches all unmatched routes
- This applies to all HTTP methods (GET, POST, PUT, DELETE, etc.)
- The fallback runs after all other routes are checked

### Response Format
- Simple plain text response: "NOT FOUND"
- HTTP status code: 404
- No JSON or HTML formatting to keep it simple

### Testing Strategy
- Unit test for the handler function itself
- Integration tests covering various scenarios:
  - Different paths
  - Different HTTP methods
  - With and without query strings
  - With and without request bodies

## Success Criteria

- [ ] 404 handler returns correct status code and message
- [ ] Requests to non-existent endpoints are properly logged
- [ ] Logging includes all required information (method, path, querystring, content length)
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Existing functionality remains unaffected