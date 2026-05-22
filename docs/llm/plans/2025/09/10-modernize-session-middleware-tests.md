# Merge Session Middleware Tests into Integration Tests

**Date:** 2025-09-10@23:50

## Problem Statement

The `tests/session_middleware_tests.rs` file is outdated and incompatible with the current architecture:

1. **Environment Variable Dependency**: Uses `setup_test_environment()` and `get_secret_from_env()` which are no longer needed since `DpAuthServer` can be initialized with manual variables via the builder pattern.

2. **Architectural Mismatch**: Tests the session middleware in isolation using custom routers, while the modern approach uses `DpAuthServer` which already integrates the session middleware.

3. **Redundant Testing**: Some tests like `test_session_middleware_can_be_applied_to_router` are redundant since `DpAuthServer` tests already verify the middleware works without crashing.

4. **Inconsistent Test Patterns**: Uses different patterns compared to `tests/integration_tests.rs` which follows the modern `DpAuthServer` approach.

5. **Maintenance Overhead**: Having separate middleware tests creates duplication and additional maintenance burden.

## Analysis

### Current Session Middleware Tests
The file contains 8 tests:
1. `test_session_middleware_can_be_applied_to_router` - Tests middleware can be applied (redundant)
2. `test_session_context_helper_methods` - Tests `SessionContext` utility methods
3. `test_session_cookie_name_constant` - Tests constant value
4. `test_session_middleware_with_valid_header_token` - Tests header token extraction
5. `test_session_middleware_with_valid_cookie_token` - Tests cookie token extraction
6. `test_session_middleware_prefers_header_over_cookie` - Tests precedence logic
7. `test_session_middleware_invalid_header_no_cookie_fallback` - Tests fallback behavior
8. `test_session_middleware_expired_token` - Tests expired token handling

### Integration Tests Pattern
The `tests/integration_tests.rs` file demonstrates the modern approach:
- Uses `DpAuthServer::new().session_secret(secret).build()` instead of environment variables
- Creates temporary databases for test isolation
- Tests end-to-end functionality through actual HTTP requests
- Already covers session functionality through `getCurrentSession` tests

### Value Assessment

**Current Integration Test Coverage:**
- ✅ Session creation via `createSession` mutation
- ✅ Session validation via `getCurrentSession` query with Authorization header
- ✅ Invalid token handling
- ✅ Unauthenticated requests

**Missing Coverage (from session middleware tests):**
- ❌ Cookie-based authentication
- ❌ Header vs cookie precedence logic
- ❌ Invalid header with valid cookie fallback behavior
- ❌ Expired token handling
- ❌ SessionContext utility methods testing
- ❌ Session cookie name constant verification

## Proposed Solution

**Merge all valuable session middleware tests into `integration_tests.rs` and delete `session_middleware_tests.rs`.**

This approach provides:
1. **Unified Testing Strategy**: All tests follow the same `DpAuthServer` pattern
2. **End-to-End Coverage**: Tests verify actual behavior through real HTTP requests
3. **Reduced Maintenance**: Single test file to maintain
4. **Modern Architecture**: No environment variable dependencies
5. **Better Test Isolation**: Each test uses its own `DpAuthServer` instance

## Implementation Plan

### Phase 1: Add Missing Session Tests to Integration Tests

**File to Modify:**
- `tests/integration_tests.rs`

**New Tests to Add:**

1. **`test_get_current_session_with_cookie_authentication`**
   - Create user and session via mutations
   - Extract session cookie from `Set-Cookie` header
   - Test `getCurrentSession` using cookie instead of Authorization header
   - Verify session data is returned correctly

2. **`test_session_header_precedence_over_cookie`**
   - Create two different sessions (different users)
   - Send request with both Authorization header and cookie
   - Verify that header token takes precedence over cookie

3. **`test_session_invalid_header_no_cookie_fallback`**
   - Create valid session cookie
   - Send request with invalid Authorization header and valid cookie
   - Verify that invalid header prevents cookie fallback (returns unauthenticated)

4. **`test_session_expired_token_handling`**
   - Create session with manually crafted expired token
   - Test both header and cookie scenarios
   - Verify expired tokens are rejected

5. **`test_session_context_utility_methods`**
   - Unit test for `SessionContext` helper methods
   - Test `authenticated()`, `user_id()`, etc.
   - Can be standalone unit test within integration file

6. **`test_session_cookie_name_constant`**
   - Simple test to verify `SESSION_COOKIE_NAME` constant value
   - Ensures cookie name consistency

### Phase 2: Remove Session Middleware Test File

**File to Delete:**
- `tests/session_middleware_tests.rs`

**Cargo.toml Updates:**
- Remove any test-specific dependencies that were only used by session middleware tests

### Code Samples

**Cookie Authentication Test:**
```rust
#[tokio::test]
async fn test_get_current_session_with_cookie_authentication() {
  let app = create_app().await;
  
  // Create user and session
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_mutation(&app, &unique_username, "password123").await;
  
  // Create session and extract cookie
  let create_session_response = create_session_via_mutation(&app, &unique_username, "password123").await;
  let cookie_header = create_session_response.headers().get("set-cookie").unwrap();
  
  // Test getCurrentSession with cookie
  let query = r#"{"query": "{ getCurrentSession { sub iat exp } }"}"#;
  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .header("cookie", cookie_header.to_str().unwrap())
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();
    
  assert_eq!(response.status(), StatusCode::OK);
  let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let data: serde_json::Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();
  
  assert!(data["errors"].is_null());
  assert!(!data["data"]["getCurrentSession"].is_null());
}
```

**Header Precedence Test:**
```rust
#[tokio::test]
async fn test_session_header_precedence_over_cookie() {
  let app = create_app().await;
  
  // Create two different users and sessions
  let user1 = format!("user1_{}", rand::random::<u32>());
  let user2 = format!("user2_{}", rand::random::<u32>());
  
  let user1_uuid = create_test_user_via_mutation(&app, &user1, "password123").await;
  let user2_uuid = create_test_user_via_mutation(&app, &user2, "password123").await;
  
  // Create sessions for both users
  let session1_response = create_session_via_mutation(&app, &user1, "password123").await;
  let session2_response = create_session_via_mutation(&app, &user2, "password123").await;
  
  // Extract tokens
  let header_token = extract_token_from_response(&session1_response);
  let cookie_header = session2_response.headers().get("set-cookie").unwrap();
  
  // Test with both header and cookie - header should win
  let query = r#"{"query": "{ getCurrentSession { sub iat exp } }"}"#;
  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", header_token))
        .header("cookie", cookie_header.to_str().unwrap())
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap();
    
  // Should return user1's session (from header), not user2's (from cookie)
  let data = parse_graphql_response(response).await;
  let returned_user_id = data["data"]["getCurrentSession"]["sub"].as_i64().unwrap();
  
  // Verify it matches user1's ID (header token), not user2's (cookie)
  assert_eq!(returned_user_id, extract_user_id_from_uuid(&user1_uuid));
}
```

**Utility Methods Test:**
```rust
#[test]
fn test_session_context_utility_methods() {
  use dp_auth_service::middleware::session::SessionContext;
  use dp_auth_session_service::DpAuthSessionPayload;
  
  // Test empty context
  let empty_context = SessionContext::new(None);
  assert!(!empty_context.authenticated());
  assert_eq!(empty_context.user_id(), None);
  
  // Test authenticated context
  let payload = DpAuthSessionPayload {
    sub: 123,
    iat: 1000,
    exp: 2000,
  };
  let auth_context = SessionContext::new(Some(payload));
  assert!(auth_context.authenticated());
  assert_eq!(auth_context.user_id(), Some(123));
}

#[test]
fn test_session_cookie_name_constant() {
  use dp_auth_service::middleware::session::SESSION_COOKIE_NAME;
  assert_eq!(SESSION_COOKIE_NAME, "DpAuthSession");
}
```

### Helper Functions to Add

```rust
// Helper to create session and return response (for cookie extraction)
async fn create_session_via_mutation(app: &Router, username: &str, password: &str) -> Response<Body> {
  let query = format!(
    r#"{{"query": "mutation {{ createSession(input: {{ username: \"{}\", password: \"{}\" }}) {{ token message }} }}"}}"#,
    username, password
  );
  
  app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query))
        .unwrap(),
    )
    .await
    .unwrap()
}

// Helper to extract token from GraphQL response
fn extract_token_from_response(response: &Response<Body>) -> String {
  // Parse response body and extract token field
  // Implementation details...
}
```

## Benefits

1. **Simplified Architecture**: Single test file following consistent patterns
2. **End-to-End Testing**: All tests verify actual HTTP behavior through `DpAuthServer`
3. **No Environment Dependencies**: Tests are self-contained with generated secrets
4. **Reduced Maintenance**: One test file to maintain instead of two
5. **Better Coverage**: Tests verify real-world scenarios through actual HTTP requests
6. **Modern Patterns**: Follows the established `integration_tests.rs` approach

## Risks

1. **Test Complexity**: Integration tests are more complex than unit tests
2. **Slower Execution**: End-to-end tests may be slower than isolated middleware tests
3. **Debugging**: Failures may be harder to isolate compared to unit tests

## Migration Strategy

1. **Phase 1**: Add all missing session test coverage to `integration_tests.rs`
2. **Phase 2**: Verify all tests pass and provide equivalent coverage
3. **Phase 3**: Delete `tests/session_middleware_tests.rs`
4. **Phase 4**: Update documentation and CI if needed

## Files Summary

**Files to be Modified:**
- `tests/integration_tests.rs` - Add comprehensive session testing coverage

**Files to be Deleted:**
- `tests/session_middleware_tests.rs` - Remove outdated middleware tests

**Files to be Created:**
- None

**Dependencies:**
- No new dependencies required (all needed dependencies already in `integration_tests.rs`)