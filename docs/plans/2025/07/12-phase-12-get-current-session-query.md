# Phase 12: `getCurrentSession` Query

**Date**: 2025-07-12@09:32

## Overview

This phase implements the `getCurrentSession` GraphQL query that returns the current session token payload set by the session middleware. This query allows authenticated clients to retrieve information about their current session, including user ID and token expiration details.

## Requirements from README

- [ ] Returns the current session token payload, set by the session middleware
- [ ] Make a decision if the query should return 2XX or 401 status code if the session token is not present or invalid
- [ ] Unit tests for the `getCurrentSession` query
- [ ] Integration tests for the `getCurrentSession` query

## Current State Analysis

### Existing Infrastructure
- ✅ `SessionService` with `SessionPayload` struct and `decode_token` method
- ✅ Session middleware that extracts tokens and attaches `SessionContext` to requests
- ✅ `SessionContext::from_context()` helper method for accessing session data in resolvers
- ✅ GraphQL query structure established with `GetServerTimestampQuery` as reference
- ✅ Error handling patterns established in existing queries

### SessionPayload Structure
The `SessionPayload` struct contains:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionPayload {
  /// Subject - user ID
  pub sub: i64,
  /// Issued at timestamp (seconds since epoch)
  pub iat: i64,
  /// Expiration timestamp (seconds since epoch)
  pub exp: i64,
}
```

### SessionContext Helper
The session middleware provides a `SessionContext` with:
```rust
pub struct SessionContext {
  pub payload: Option<SessionPayload>,
}

impl SessionContext {
  pub fn from_context<'a>(ctx: &'a Context<'a>) -> Result<&'a SessionContext, async_graphql::Error>
}
```

## Design Decisions

### HTTP Status Code Decision
**Decision**: Return 2XX status codes for all responses, including unauthenticated requests.

**Rationale**:
- Consistent with existing GraphQL patterns in the codebase (e.g., `createSession` mutation)
- GraphQL best practices suggest using 2XX for successful query execution, with errors in the response body
- Allows clients to handle authentication state through the response data rather than HTTP status codes
- Simplifies client-side error handling by keeping authentication errors in the GraphQL error format

### Response Format
- **Authenticated**: Return the session payload with user ID and timestamps
- **Unauthenticated**: Return `null` for the session payload (not an error)
- **Internal Error**: Return GraphQL error for middleware configuration issues

## Implementation Plan

### 1. Create SessionPayload GraphQL Type
Create a GraphQL output type that mirrors the `SessionPayload` struct.

#### Rationale for Custom GraphQL Type

We create a separate GraphQL type instead of directly using the service's `SessionPayload` struct for several important reasons:

**Separation of Concerns**: The service layer's `SessionPayload` is designed for internal business logic (token encoding, database operations, internal service communication), while the GraphQL type is designed for API consumption (schema definition, client-facing documentation, API contracts).

**API Stability and Evolution**: By creating a separate GraphQL type, we can change internal implementation without breaking the API contract, add internal fields to the service struct that shouldn't be exposed to clients, modify field types in the service layer while maintaining GraphQL compatibility, and version the API independently from internal data structures.

**GraphQL-Specific Features**: The custom type allows us to add GraphQL-specific documentation via `#[doc]` comments that appear in schema introspection, apply GraphQL-specific attributes like `#[graphql(skip)]` or custom field names, implement custom resolvers for computed fields if needed in the future, and control field nullability and types specifically for the GraphQL schema.

**Future Flexibility**: This pattern allows us to easily add computed fields (e.g., `isExpired: Boolean`, `timeUntilExpiry: Int`), transform field formats (e.g., convert timestamps to ISO strings), add client-friendly fields without cluttering the service layer, and implement field-level permissions or filtering.

**Industry Best Practices**: This follows the "API Gateway Pattern" where internal services use optimized data structures, the API layer presents client-friendly interfaces, and a conversion layer handles the mapping between them. This is standard practice in GraphQL APIs at companies like GitHub, Shopify, and others who maintain long-term API stability while evolving internal implementations.

```rust
// src/graphql/types/session_payload.rs
use async_graphql::SimpleObject;
use crate::services::SessionPayload as ServiceSessionPayload;

#[derive(SimpleObject)]
pub struct SessionPayload {
  /// Subject - user ID
  pub sub: i64,
  /// Issued at timestamp (seconds since epoch)  
  pub iat: i64,
  /// Expiration timestamp (seconds since epoch)
  pub exp: i64,
}

impl From<ServiceSessionPayload> for SessionPayload {
  fn from(payload: ServiceSessionPayload) -> Self {
    Self {
      sub: payload.sub,
      iat: payload.iat,
      exp: payload.exp,
    }
  }
}
```

### 2. Implement GetCurrentSessionQuery
Create the query resolver following the established patterns:

```rust
// src/graphql/queries/get_current_session.rs
use crate::graphql::types::SessionPayload;
use crate::middleware::SessionContext;
use async_graphql::{Context, Object, Result};
use tracing::instrument;

/// Current session query resolver providing authenticated user session information
#[derive(Default, Debug)]
pub struct GetCurrentSessionQuery;

#[Object]
impl GetCurrentSessionQuery {
  /// Returns the current session payload if the user is authenticated.
  ///
  /// This query retrieves the session information that was set by the session middleware
  /// when processing the request. The session contains:
  /// - User ID (sub): The unique identifier of the authenticated user
  /// - Issued at (iat): When the session token was created (seconds since epoch)
  /// - Expiration (exp): When the session token expires (seconds since epoch)
  ///
  /// Returns `null` if no valid session token was provided in the request.
  ///
  /// # Authentication
  /// 
  /// This query requires a valid session token to be provided either:
  /// - In the `Authorization` header as `Bearer <token>`
  /// - In the `DpAuthSession` cookie
  ///
  /// # Examples
  ///
  /// **Authenticated request:**
  /// ```graphql
  /// query {
  ///   getCurrentSession {
  ///     sub
  ///     iat
  ///     exp
  ///   }
  /// }
  /// ```
  ///
  /// **Response for authenticated user:**
  /// ```json
  /// {
  ///   "data": {
  ///     "getCurrentSession": {
  ///       "sub": 123,
  ///       "iat": 1706356800,
  ///       "exp": 1706616000
  ///     }
  ///   }
  /// }
  /// ```
  ///
  /// **Response for unauthenticated user:**
  /// ```json
  /// {
  ///   "data": {
  ///     "getCurrentSession": null
  ///   }
  /// }
  /// ```
  #[instrument]
  async fn get_current_session(&self, ctx: &Context<'_>) -> Result<Option<SessionPayload>> {
    let session_context = SessionContext::from_context(ctx)?;
    
    match &session_context.payload {
      Some(payload) => Ok(Some(payload.clone().into())),
      None => Ok(None),
    }
  }
}
```

### 3. Update Module Structure
Update the necessary module files to include the new query:

```rust
// src/graphql/types/mod.rs (new file)
pub mod session_payload;
pub use session_payload::SessionPayload;

// src/graphql/queries/mod.rs
pub mod get_current_session;
pub mod get_server_timestamp;

pub use get_current_session::GetCurrentSessionQuery;
pub use get_server_timestamp::GetServerTimestampQuery;

// src/graphql/query.rs
use crate::graphql::queries::{GetCurrentSessionQuery, GetServerTimestampQuery};
use async_graphql::{MergedObject, Object};

#[derive(MergedObject, Default)]
pub struct Query(GetServerTimestampQuery, GetCurrentSessionQuery);

impl Query {
  pub fn new() -> Self {
    Self::default()
  }
}
```

### 4. Unit Tests
Create comprehensive unit tests for the query resolver:

```rust
#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::SessionContext;
  use crate::services::SessionPayload as ServiceSessionPayload;
  use async_graphql::*;

  #[tokio::test]
  async fn test_get_current_session_with_authenticated_user() {
    let query = GetCurrentSessionQuery;
    let payload = ServiceSessionPayload {
      sub: 123,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(payload.clone()));
    
    let schema = Schema::build(query, EmptyMutation, EmptySubscription)
      .data(session_context)
      .finish();
      
    let result = schema.execute("{ getCurrentSession { sub iat exp } }").await;
    
    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["getCurrentSession"]["sub"], 123);
    assert_eq!(data["getCurrentSession"]["iat"], 1706356800);
    assert_eq!(data["getCurrentSession"]["exp"], 1706616000);
  }

  #[tokio::test]
  async fn test_get_current_session_with_unauthenticated_user() {
    let query = GetCurrentSessionQuery;
    let session_context = SessionContext::new(None);
    
    let schema = Schema::build(query, EmptyMutation, EmptySubscription)
      .data(session_context)
      .finish();
      
    let result = schema.execute("{ getCurrentSession { sub iat exp } }").await;
    
    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert!(data["getCurrentSession"].is_null());
  }

  #[tokio::test]
  async fn test_get_current_session_missing_context() {
    let query = GetCurrentSessionQuery;
    
    let schema = Schema::build(query, EmptyMutation, EmptySubscription).finish();
    let result = schema.execute("{ getCurrentSession { sub iat exp } }").await;
    
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Internal server error"));
  }
}
```

### 5. Integration Tests
Create integration tests that test the complete flow:

```rust
// In tests/integration_tests.rs or a new test file
#[tokio::test]
async fn test_get_current_session_integration_authenticated() {
  let app = create_test_app().await;
  
  // First create a user and get a session token
  let create_user_response = app
    .post("/graphql")
    .json(&json!({
      "query": "mutation { createUser(username: \"testuser\", password: \"password123\") { id username } }"
    }))
    .send()
    .await;
  assert_eq!(create_user_response.status(), 200);
  
  // Create session
  let create_session_response = app
    .post("/graphql")
    .json(&json!({
      "query": "mutation { createSession(username: \"testuser\", password: \"password123\") { token } }"
    }))
    .send()
    .await;
  assert_eq!(create_session_response.status(), 200);
  
  let session_data: serde_json::Value = create_session_response.json().await;
  let token = session_data["data"]["createSession"]["token"].as_str().unwrap();
  
  // Test getCurrentSession with token
  let response = app
    .post("/graphql")
    .header("Authorization", format!("Bearer {}", token))
    .json(&json!({
      "query": "{ getCurrentSession { sub iat exp } }"
    }))
    .send()
    .await;
    
  assert_eq!(response.status(), 200);
  let data: serde_json::Value = response.json().await;
  assert!(data["errors"].is_null());
  assert!(!data["data"]["getCurrentSession"].is_null());
  assert!(data["data"]["getCurrentSession"]["sub"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_get_current_session_integration_unauthenticated() {
  let app = create_test_app().await;
  
  let response = app
    .post("/graphql")
    .json(&json!({
      "query": "{ getCurrentSession { sub iat exp } }"
    }))
    .send()
    .await;
    
  assert_eq!(response.status(), 200);
  let data: serde_json::Value = response.json().await;
  assert!(data["errors"].is_null());
  assert!(data["data"]["getCurrentSession"].is_null());
}

#[tokio::test]
async fn test_get_current_session_integration_invalid_token() {
  let app = create_test_app().await;
  
  let response = app
    .post("/graphql")
    .header("Authorization", "Bearer invalid_token")
    .json(&json!({
      "query": "{ getCurrentSession { sub iat exp } }"
    }))
    .send()
    .await;
    
  assert_eq!(response.status(), 200);
  let data: serde_json::Value = response.json().await;
  assert!(data["errors"].is_null());
  assert!(data["data"]["getCurrentSession"].is_null());
}
```

## Files to Create/Modify

### New Files
1. `src/graphql/types/mod.rs` - Module for GraphQL types
2. `src/graphql/types/session_payload.rs` - GraphQL type for session payload
3. `src/graphql/queries/get_current_session.rs` - Query resolver implementation

### Modified Files
1. `src/graphql/mod.rs` - Add types module export
2. `src/graphql/queries/mod.rs` - Export new query
3. `src/graphql/query.rs` - Add new query to merged object
4. `tests/integration_tests.rs` - Add integration tests

## Dependencies

- No new external dependencies required
- Relies on existing session middleware and `SessionContext`
- Uses established GraphQL patterns from existing queries

## Testing Strategy

### Unit Tests
- Test authenticated user scenario with valid session payload
- Test unauthenticated user scenario (no session payload)
- Test missing session context (middleware configuration error)
- Verify proper conversion from service types to GraphQL types

### Integration Tests
- Test complete flow with valid session token in header
- Test complete flow with valid session token in cookie
- Test unauthenticated request (no token)
- Test invalid token scenarios
- Test expired token scenarios

## Security Considerations

- Query returns `null` for unauthenticated users rather than an error, preventing information leakage
- Session validation is handled entirely by the middleware layer
- No sensitive information beyond what's already in the session token is exposed
- Follows principle of least privilege by only returning session metadata

## Future Enhancements

- Add session metadata like IP address, user agent, or last activity
- Implement session refresh mechanism
- Add session revocation capabilities
- Support for multiple concurrent sessions per user