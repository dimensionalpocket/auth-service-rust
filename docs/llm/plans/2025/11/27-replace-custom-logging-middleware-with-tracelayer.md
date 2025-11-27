# Plan: Replace Custom Logging Middleware with TraceLayer

## Current Issues
1. `rest_logging_middleware` only logs specific paths (`/`, `/health`) and 404s
2. GraphQL GET/POST requests to `/api/graphql` are not logged
3. Custom middleware is redundant when tower-http provides a robust solution

## Current Logging Analysis

### What's Currently Working
- **GraphQL POST requests**: Logged by `async_graphql::extensions::Tracing` + `#[instrument]` on handler
- **REST endpoints**: `/` and `/health` logged by custom middleware
- **404s**: Logged by custom middleware

### What's Missing
- **GraphQL GET requests**: Not logged at all (bypasses both logging systems)

### Logging Layers
1. **HTTP-level** (missing): Method, URI, status, duration for ALL requests
2. **GraphQL-level** (working): Query parsing, field resolution via async-graphql Tracing

## Solution
Replace the custom logging middleware with tower-http's `TraceLayer::new_for_http()` which will add **HTTP-level logging**:
- All HTTP requests (GET, POST, etc.) to all endpoints
- All responses including status codes and duration  
- 404 errors and any other status codes
- **GraphQL GET requests** (currently missing)
- **GraphQL POST requests** (additional HTTP context to existing GraphQL logs)

The existing `async_graphql::extensions::Tracing` will continue to provide GraphQL operation-level logging. These are complementary:
- **TraceLayer**: HTTP request/response details
- **GraphQL Tracing**: GraphQL execution details

## Implementation Steps

### 1. Update Cargo.toml
- Ensure `tower-http` has the `trace` feature enabled
- Current: `tower-http = { version = "0.5", features = ["cors"] }`
- Update to: `tower-http = { version = "0.5", features = ["cors", "trace"] }`

### 2. Update src/dps_auth_api.rs
- Remove the custom logging middleware layer (line 214)
- Add `TraceLayer::new_for_http()` to the ServiceBuilder
- Import `tower_http::trace::TraceLayer`

### 3. Remove src/middleware/logging.rs
- Delete the entire file as it's no longer needed

### 4. Update src/middleware/mod.rs
- Remove the `pub mod logging;` line

### 5. Extract logging tests to dedicated file
- Create `tests/logging_tests.rs` for all logging-related tests
- Move password logging security tests from `tests/integration_tests.rs`
- Move shutdown service logging tests from `src/services/shutdown_service.rs`

### 6. Update tests for new logging behavior
- Update test assertions to work with TraceLayer structured logging
- Ensure password logging tests still work with new HTTP logging

## Benefits
- **Comprehensive**: Logs ALL HTTP requests and responses
- **Structured**: Uses tracing spans for better observability
- **Standard**: Uses battle-tested tower-http middleware
- **Configurable**: Can be controlled via standard tracing filters
- **Performance**: Optimized implementation from tower-http
- **Maintenance**: Less custom code to maintain

## Example Log Output

### After Implementation (Combined Logging)
**GraphQL GET Request** (currently missing):
```
INFO request{method=GET uri=/api/graphql?query={authMe} ...}: tower_http::trace::on_request: started processing request
INFO request{method=GET uri=/api/graphql?query={authMe} ...}: tower_http::trace::on_response: finished processing request
INFO graphql_handler{...}: async_graphql::extensions::tracing: GraphQL execution details
```

**GraphQL POST Request** (enhanced):
```
INFO request{method=POST uri=/api/graphql ...}: tower_http::trace::on_request: started processing request  
INFO request{method=POST uri=/api/graphql ...}: tower_http::trace::on_response: finished processing request
INFO graphql_handler{...}: async_graphql::extensions::tracing: GraphQL execution details
```

**REST Request** (same as before):
```
INFO request{method=GET uri=/health ...}: tower_http::trace::on_request: started processing request
INFO request{method=GET uri=/health ...}: tower_http::trace::on_response: finished processing request
```

**404 Request** (same as before):
```
INFO request{method=GET uri=/nonexistent ...}: tower_http::trace::on_request: started processing request
INFO request{method=GET uri=/nonexistent ...}: tower_http::trace::on_response: finished processing request
```

## Configuration
The logging level can be controlled via environment variables:
- `RUST_LOG=tower_http=debug` for verbose HTTP logging
- `RUST_LOG=tower_http=info` for standard HTTP logging
- Current config already sets `tower_http=debug` in `run_local_server.rs`

## Test Files to Modify

### Files to Change
1. **tests/integration_tests.rs**
   - Remove: `test_auth_login_no_password_in_logs()` (lines 712-746)
   - Remove: `test_auth_register_no_password_in_logs()` (lines 748-779)
   - These will be moved to `tests/logging_tests.rs`

2. **src/services/shutdown_service.rs**
   - Remove: Entire test module (lines 46-95)
   - Move 3 test functions to `tests/logging_tests.rs`
   - Keep: `logs_contain()` helper function (needs to be accessible)

3. **Create: tests/logging_tests.rs**
   - Add password logging security tests (from integration_tests.rs)
   - Add shutdown service logging tests (from shutdown_service.rs)
   - Add new HTTP logging tests for TraceLayer
   - Add helper functions and imports

### New Test Structure
```
tests/
├── logging_tests.rs          # NEW: All logging-related tests
├── integration_tests.rs       # MODIFIED: Remove logging tests
├── database_integration_tests.rs
├── get_query_support_tests.rs
└── playground_tests.rs
```

### Test Dependencies
- Ensure `tracing_test` is available in new test file
- May need to add `use` statements for helper functions
- Consider creating a `tests/common.rs` for shared test utilities

## Customization

The TraceLayer can be customized for different logging behaviors:

### Default Behavior
- Non-POST requests: DEBUG level
- POST requests: INFO level

### Custom Configuration (Implemented)
All requests now log at INFO level:
```rust
.layer(
  TraceLayer::new_for_http()
    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
    .on_request(DefaultOnRequest::new().level(Level::INFO))
    .on_response(DefaultOnResponse::new().level(Level::INFO))
)
```

### Further Customization Options
- `DefaultMakeSpan::level()` - Set span level
- `DefaultMakeSpan::include_headers()` - Include request headers
- `DefaultOnRequest::level()` - Set request event level
- `DefaultOnResponse::level()` - Set response event level
- Custom classifiers for different response handling
- Custom span creation logic

### Result
✅ **All HTTP methods (GET, POST, PUT, DELETE, etc.) now log at INFO level**
✅ **GraphQL GET requests now properly logged** (solves original issue)
✅ **Comprehensive HTTP request logging** for all endpoints including 404s
✅ **Customizable logging behavior** for different use cases

This approach solves the original issue of GraphQL GET requests not being logged, while providing comprehensive HTTP request logging for all endpoints including 404s, organizing tests more cleanly, and allowing customizable logging behavior.