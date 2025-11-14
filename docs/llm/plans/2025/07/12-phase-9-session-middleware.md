# Phase 9: Session Middleware

**Date**: 2025-07-12  
**Phase**: 9 of 12  
**Goal**: Implement middleware for session token management in GraphQL API with request context integration

## Overview

This phase implements session middleware that will handle session token extraction, validation, and context attachment for all GraphQL queries and mutations. The middleware will check for session tokens in request headers or cookies, decode them using the existing `SessionService`, and attach the session payload to the request context for use by GraphQL resolvers.

## Design Decisions

### Authentication Header Prefix
**Decision**: Use `"Bearer"` as the header prefix for consistency with industry standards, even though we're using custom encrypted tokens instead of JWT.
- **Rationale**: Most client libraries and tools expect `Authorization: Bearer <token>` format
- **Header Format**: `Authorization: Bearer <encrypted_token>`

### Cookie Name
**Decision**: Use `"DpAuthSession"` as the cookie name
- **Rationale**: Clear, descriptive name that follows naming conventions
- **Constant**: Store in `SESSION_COOKIE_NAME` constant for reusability

### Token Priority
**Decision**: Prefer header over cookie when both are present
- **Rationale**: Headers are more explicit and secure for API usage
- **Behavior**: If header token is invalid, do NOT fallback to cookie

### Error Handling
**Decision**: Allow requests to proceed without session when token is missing or invalid
- **Rationale**: Each resolver can decide if authentication is required
- **Behavior**: Attach `None` to context instead of returning error responses

## Tasks Breakdown

### 1. Session Context Structure
Create a context structure to hold the session payload that can be attached to GraphQL requests.

**File**: `src/middleware/session.rs`
```rust
use crate::services::SessionPayload;

/// Session context that gets attached to GraphQL requests
#[derive(Debug, Clone)]
pub struct SessionContext {
  pub payload: Option<SessionPayload>,
}

impl SessionContext {
  pub fn new(payload: Option<SessionPayload>) -> Self {
    Self { payload }
  }
  
  pub fn authenticated(&self) -> bool {
    self.payload.is_some()
  }
  
  pub fn user_id(&self) -> Option<i64> {
    self.payload.as_ref().map(|p| p.sub)
  }
}
```

### 2. Session Middleware Implementation
Implement the core middleware logic for token extraction and validation.

**File**: `src/middleware/session.rs` (continued)
```rust
use axum::{extract::Request, middleware::Next, response::Response};
use crate::services::SessionService;

/// Cookie name for session tokens
pub const SESSION_COOKIE_NAME: &str = "DpAuthSession";

/// Session middleware for GraphQL requests
pub async fn session_middleware(mut request: Request, next: Next) -> Response {
  let session_context = extract_and_validate_session(&request).await;
  
  // Attach session context to request extensions
  request.extensions_mut().insert(session_context);
  
  next.run(request).await
}

/// Extract session token from request and validate it
async fn extract_and_validate_session(request: &Request) -> SessionContext {
  // Try header first
  if let Some(token) = extract_token_from_header(request) {
    if let Ok(payload) = SessionService::decode_token(&token) {
      return SessionContext::new(Some(payload));
    }
    // If header token is invalid, don't try cookie
    return SessionContext::new(None);
  }
  
  // Try cookie if no header
  if let Some(token) = extract_token_from_cookie(request) {
    if let Ok(payload) = SessionService::decode_token(&token) {
      return SessionContext::new(Some(payload));
    }
  }
  
  SessionContext::new(None)
}

/// Extract token from Authorization header
fn extract_token_from_header(request: &Request) -> Option<String> {
  request
    .headers()
    .get("authorization")?
    .to_str()
    .ok()?
    .strip_prefix("Bearer ")
    .map(|token| token.to_string())
}

/// Extract token from cookie
fn extract_token_from_cookie(request: &Request) -> Option<String> {
  let cookie_header = request.headers().get("cookie")?.to_str().ok()?;
  
  for cookie in cookie_header.split(';') {
    let cookie = cookie.trim();
    if let Some(value) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
      return Some(value.to_string());
    }
  }
  
  None
}
```

### 3. GraphQL Context Integration
Modify the GraphQL handler to make session context available to resolvers.

**File**: `src/handlers/graphql.rs` (modifications)
```rust
// Add import
use crate::middleware::session::SessionContext;

// Modify graphql_post_handler function
pub async fn graphql_post_handler(
  State(schema): State<AppSchema>,
  req: GraphQLRequest,
) -> impl IntoResponse {
  let start = Instant::now();
  
  let mut request = req.into_inner();
  
  // Extract session context from request extensions
  let session_context = req.extensions()
    .get::<SessionContext>()
    .cloned()
    .unwrap_or_else(|| SessionContext::new(None));
  
  // Add session context to GraphQL request data
  request = request.data(session_context);
  
  // ... rest of the function remains the same
}
```

### 4. Middleware Registration
Update the main application to use session middleware only for GraphQL endpoints.

**File**: `src/main.rs` (modifications)
```rust
// Add import
use dp_auth_service::middleware::session::session_middleware;

// Modify the router setup
let app = Router::new()
  .route("/", get(root_handler))
  .route("/health", get(health_handler))
  .route(
    "/graphql",
    get(graphql_get_handler).post(graphql_post_handler)
      .layer(middleware::from_fn(session_middleware)), // Add session middleware only to GraphQL
  )
  .layer(
    ServiceBuilder::new()
      .layer(middleware::from_fn(request_id_middleware))
      .layer(middleware::from_fn(rest_logging_middleware))
      .layer(CorsLayer::permissive()),
  )
  .with_state(schema);
```

### 5. Resolver Context Access
Create helper functions for resolvers to access session context.

**File**: `src/middleware/session.rs` (continued)
```rust
use async_graphql::Context;

impl SessionContext {
  /// Get session context from GraphQL context
  pub fn from_graphql_context(ctx: &Context<'_>) -> Option<&SessionContext> {
    ctx.data_opt::<SessionContext>()
  }
  
  /// Get session context from GraphQL context with error logging
  /// 
  /// This is the recommended method for resolvers as it logs missing context
  /// as a server error and returns a user-friendly error message.
  pub fn from_graphql_context_or_error(ctx: &Context<'_>) -> Result<&SessionContext, async_graphql::Error> {
    match ctx.data_opt::<SessionContext>() {
      Some(context) => Ok(context),
      None => {
        tracing::error!("Session context not available in GraphQL resolver - middleware may not be configured properly");
        Err("Internal server error".into())
      }
    }
  }
}
```

### 6. Module Updates
Update module declarations to include the new session middleware.

**File**: `src/middleware/mod.rs`
```rust
pub mod logging;
pub mod request_id;
pub mod session;
```

**File**: `src/lib.rs` (ensure middleware module is exported)
```rust
pub mod middleware;
// ... other modules
```

## Implementation Files Summary

### Files to Create
1. **`src/middleware/session.rs`** - Complete session middleware implementation

### Files to Modify
1. **`src/middleware/mod.rs`** - Add session module
2. **`src/handlers/graphql.rs`** - Integrate session context with GraphQL requests
3. **`src/main.rs`** - Register session middleware for GraphQL endpoints

## Unit Tests

### Test Structure
**File**: `tests/middleware/session_tests.rs`

```rust
use dp_auth_service::{
  middleware::session::{session_middleware, SessionContext, SESSION_COOKIE_NAME},
  services::{SessionService, SessionPayload},
};
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use std::env;

#[tokio::test]
async fn test_session_middleware_with_valid_header_token() {
  // Setup secret key
  env::set_var("DP_AUTH_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");
  
  // Create valid token
  let payload = SessionService::create_payload(123);
  let token = SessionService::encode_token(&payload).unwrap();
  
  // Create request with Authorization header
  let mut request = Request::builder()
    .header("authorization", format!("Bearer {}", token))
    .body(Body::empty())
    .unwrap();
  
  // Run middleware
  let response = session_middleware(request, |req| async move {
    let session_context = req.extensions().get::<SessionContext>().unwrap();
    assert!(session_context.authenticated());
    assert_eq!(session_context.user_id(), Some(123));
    Response::new(Body::empty())
  }).await;
  
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_with_valid_cookie_token() {
  // Setup secret key
  env::set_var("DP_AUTH_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");
  
  // Create valid token
  let payload = SessionService::create_payload(456);
  let token = SessionService::encode_token(&payload).unwrap();
  
  // Create request with cookie
  let mut request = Request::builder()
    .header("cookie", format!("{}={}", SESSION_COOKIE_NAME, token))
    .body(Body::empty())
    .unwrap();
  
  // Run middleware
  let response = session_middleware(request, |req| async move {
    let session_context = req.extensions().get::<SessionContext>().unwrap();
    assert!(session_context.authenticated());
    assert_eq!(session_context.user_id(), Some(456));
    Response::new(Body::empty())
  }).await;
  
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_prefers_header_over_cookie() {
  // Setup secret key
  env::set_var("DP_AUTH_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");
  
  // Create two different valid tokens
  let header_payload = SessionService::create_payload(123);
  let header_token = SessionService::encode_token(&header_payload).unwrap();
  
  let cookie_payload = SessionService::create_payload(456);
  let cookie_token = SessionService::encode_token(&cookie_payload).unwrap();
  
  // Create request with both header and cookie
  let mut request = Request::builder()
    .header("authorization", format!("Bearer {}", header_token))
    .header("cookie", format!("{}={}", SESSION_COOKIE_NAME, cookie_token))
    .body(Body::empty())
    .unwrap();
  
  // Run middleware
  let response = session_middleware(request, |req| async move {
    let session_context = req.extensions().get::<SessionContext>().unwrap();
    assert!(session_context.authenticated());
    // Should use header token (user 123), not cookie token (user 456)
    assert_eq!(session_context.user_id(), Some(123));
    Response::new(Body::empty())
  }).await;
  
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_invalid_header_no_cookie_fallback() {
  // Setup secret key
  env::set_var("DP_AUTH_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");
  
  // Create valid cookie token
  let cookie_payload = SessionService::create_payload(456);
  let cookie_token = SessionService::encode_token(&cookie_payload).unwrap();
  
  // Create request with invalid header and valid cookie
  let mut request = Request::builder()
    .header("authorization", "Bearer invalid-token")
    .header("cookie", format!("{}={}", SESSION_COOKIE_NAME, cookie_token))
    .body(Body::empty())
    .unwrap();
  
  // Run middleware
  let response = session_middleware(request, |req| async move {
    let session_context = req.extensions().get::<SessionContext>().unwrap();
    // Should not be authenticated (no fallback to cookie)
    assert!(!session_context.authenticated());
    assert_eq!(session_context.user_id(), None);
    Response::new(Body::empty())
  }).await;
  
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_no_token() {
  // Create request without any tokens
  let mut request = Request::builder()
    .body(Body::empty())
    .unwrap();
  
  // Run middleware
  let response = session_middleware(request, |req| async move {
    let session_context = req.extensions().get::<SessionContext>().unwrap();
    assert!(!session_context.authenticated());
    assert_eq!(session_context.user_id(), None);
    Response::new(Body::empty())
  }).await;
  
  assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_session_middleware_expired_token() {
  // Setup secret key
  env::set_var("DP_AUTH_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");
  
  // Create expired token (manually create payload with past expiration)
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs() as i64;
  
  let expired_payload = SessionPayload {
    sub: 123,
    iat: now - 3600, // 1 hour ago
    exp: now - 1800, // 30 minutes ago (expired)
  };
  
  let token = SessionService::encode_token(&expired_payload).unwrap();
  
  // Create request with expired token
  let mut request = Request::builder()
    .header("authorization", format!("Bearer {}", token))
    .body(Body::empty())
    .unwrap();
  
  // Run middleware
  let response = session_middleware(request, |req| async move {
    let session_context = req.extensions().get::<SessionContext>().unwrap();
    assert!(!session_context.authenticated());
    assert_eq!(session_context.user_id(), None);
    Response::new(Body::empty())
  }).await;
  
  assert_eq!(response.status(), 200);
}
```

## Integration Testing

Integration tests will be implemented in later phases when GraphQL resolvers actually use the session context. The unit tests above verify that the middleware correctly attaches session context to requests.

## Documentation

### Rust Doc Comments
All public functions and structs will include comprehensive documentation with examples showing how resolvers can access session context.

### API Documentation
Update `docs/api.md` to document:
- Session token format in headers and cookies
- How resolvers access session context
- Authentication requirements for future endpoints

## Dependencies

No new dependencies required. The implementation uses:
- Existing `SessionService` for token validation
- `axum` middleware system
- `async-graphql` context system

## Success Criteria

- [ ] Session middleware extracts tokens from headers and cookies correctly
- [ ] Session middleware prefers header over cookie when both present
- [ ] Session middleware does not fallback to cookie if header token is invalid
- [ ] Session context is properly attached to GraphQL requests
- [ ] Resolvers can access session context via GraphQL context
- [ ] All unit tests pass with 100% coverage
- [ ] Invalid/missing tokens allow requests to proceed with empty session context
- [ ] Expired tokens are properly rejected

## Resolver Usage Example

Here's how GraphQL resolvers will access and use the session context:

### Example: Authenticated Query
```rust
use async_graphql::{Context, Object, Result};
use crate::middleware::session::SessionContext;
use crate::services::UserService;

#[Object]
impl Query {
  /// Get current user profile (requires authentication)
  async fn get_current_user(&self, ctx: &Context<'_>) -> Result<Option<User>> {
    // Get session context from GraphQL context
    let session_context = SessionContext::from_graphql_context_or_error(ctx)?;
    
    // Check if user is authenticated
    if !session_context.authenticated() {
      return Err("Authentication required".into());
    }
    
    // Get user ID from session
    let user_id = session_context.user_id().unwrap();
    
    // Call service with user ID
    match UserService::get_user_by_id(user_id).await {
      Ok(Some(user)) => Ok(Some(user)),
      Ok(None) => Err("User not found".into()),
      Err(e) => Err(format!("Database error: {}", e).into()),
    }
  }
  
  /// Get public server info (no authentication required)
  async fn get_server_info(&self, ctx: &Context<'_>) -> Result<ServerInfo> {
    // Optional: Access session context for logging/analytics
    let session_context = SessionContext::from_graphql_context_or_error(ctx)?;
    
    if let Some(user_id) = session_context.user_id() {
      tracing::info!("Server info requested by user {}", user_id);
    } else {
      tracing::info!("Server info requested by anonymous user");
    }
    
    // Return public information regardless of authentication
    Ok(ServerInfo {
      version: "1.0.0".to_string(),
      uptime: get_uptime(),
    })
  }
}
```

### Example: Authenticated Mutation
```rust
#[Object]
impl Mutation {
  /// Update user profile (requires authentication)
  async fn update_profile(
    &self,
    ctx: &Context<'_>,
    input: UpdateProfileInput,
  ) -> Result<User> {
    // Get session context
    let session_context = SessionContext::from_graphql_context_or_error(ctx)?;
    
    // Require authentication
    if !session_context.authenticated() {
      return Err("Authentication required".into());
    }
    
    let user_id = session_context.user_id().unwrap();
    
    // Update user profile
    match UserService::update_user_profile(user_id, input).await {
      Ok(user) => Ok(user),
      Err(e) => Err(format!("Failed to update profile: {}", e).into()),
    }
  }
}
```

### Example: Optional Authentication
```rust
#[Object]
impl Query {
  /// Get user posts (returns different data based on authentication)
  async fn get_user_posts(
    &self,
    ctx: &Context<'_>,
    user_id: i64,
  ) -> Result<Vec<Post>> {
    let session_context = SessionContext::from_graphql_context_or_error(ctx)?;
    
    // Check if requesting own posts vs others' posts
    let is_own_posts = session_context.user_id() == Some(user_id);
    
    // Call service with context about who's requesting
    match PostService::get_user_posts(user_id, is_own_posts).await {
      Ok(posts) => Ok(posts),
      Err(e) => Err(format!("Failed to get posts: {}", e).into()),
    }
  }
}
```

## Future Integration

This middleware prepares the foundation for:
- **Phase 10**: `SessionService::create_session` method
- **Phase 11**: `createSession` mutation (sign-in)
- **Phase 12**: `getCurrentSession` query
- Future authenticated mutations and queries

The session context will be available to all GraphQL resolvers, allowing them to:
1. Check if a user is authenticated
2. Get the current user ID
3. Implement authorization logic
4. Pass session information to service methods