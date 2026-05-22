# Session Extraction Patterns in GraphQL Resolvers

## Overview

This document explains how to extract and use session data in GraphQL resolvers for the DPS Auth API. The session architecture follows a clear flow: middleware → context → resolvers, enabling both authenticated-only and optional authentication patterns.

## Session Architecture

### 1. Middleware Layer (`src/middleware/session.rs`)

The session middleware extracts authentication tokens from HTTP requests and validates them:

```rust
// Session context structure
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

  // Key method for resolvers
  pub fn from_context<'a>(
    ctx: &'a Context<'a>,
  ) -> Result<&'a SessionContext, async_graphql::Error> {
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

### 2. GraphQL Handler (`src/handlers/graphql.rs`)

The handler extracts session context from HTTP request extensions and injects it into the GraphQL context:

```rust
// Extract session context from HTTP request extensions (set by middleware)
let session_context = http_req
  .extensions()
  .get::<SessionContext>()
  .cloned()
  .unwrap_or_else(|| SessionContext::new(None));

// Add session context to GraphQL request data
let mut request = graphql_request;
request = request.data(session_context);
```

### 3. GraphQL Types (`src/graphql/types/session_payload.rs`)

The GraphQL type represents session data:

```rust
#[derive(SimpleObject)]
pub struct SessionPayload {
  /// The unique identifier of the authenticated user
  pub sub: i64,
  /// When this session was created (seconds since Unix epoch)
  pub iat: i64,
  /// When this session expires (seconds since Unix epoch)
  pub exp: i64,
}
```

## Step-by-Step Guide for New Resolvers

### Pattern 1: Authenticated-Only Resolvers

For resolvers that require authentication:

```rust
use crate::middleware::session::SessionContext;
use async_graphql::{Context, Object, Result};

#[derive(Default)]
pub struct MyAuthenticatedResolver;

#[Object]
impl MyAuthenticatedResolver {
  async fn protected_operation(&self, ctx: &Context<'_>) -> Result<String> {
    // Extract session context
    let session_context = SessionContext::from_context(ctx)?;
    
    // Check authentication
    if !session_context.authenticated() {
      return Err("Authentication required".into());
    }
    
    // Get user ID
    let user_id = session_context.user_id()
      .ok_or("Failed to extract user ID")?;
    
    // Perform authenticated operation
    Ok(format!("Operation performed for user {}", user_id))
  }
}
```

### Pattern 2: Optional Authentication Resolvers

For resolvers that work with or without authentication:

```rust
use crate::middleware::session::SessionContext;
use async_graphql::{Context, Object, Result};

#[derive(Default)]
pub struct MyOptionalAuthResolver;

#[Object]
impl MyOptionalAuthResolver {
  async fn flexible_operation(&self, ctx: &Context<'_>) -> Result<String> {
    // Extract session context
    let session_context = SessionContext::from_context(ctx)?;
    
    match session_context.user_id() {
      Some(user_id) => {
        // Authenticated path
        Ok(format!("Authenticated operation for user {}", user_id))
      }
      None => {
        // Unauthenticated path
        Ok("Public operation".to_string())
      }
    }
  }
}
```

### Pattern 3: User ID Extraction Helper

Create a helper function for common patterns:

```rust
// In your resolver file
fn extract_user_id(ctx: &Context<'_>) -> Result<i64> {
  let session_context = SessionContext::from_context(ctx)?;
  
  session_context.user_id()
    .ok_or_else(|| "Authentication required".into())
}

// Usage in resolver
async fn user_specific_operation(&self, ctx: &Context<'_>) -> Result<MyData> {
  let user_id = extract_user_id(ctx)?;
  
  // Use user_id for database queries, etc.
  MyService::get_user_data(user_id).await
    .map_err(|e| format!("Failed to get user data: {}", e).into())
}
```

## Code Examples from Existing Codebase

### Example 1: Get Current Session (`src/graphql/resolvers/get_current_session.rs`)

```rust
async fn get_current_session(&self, ctx: &Context<'_>) -> Result<Option<SessionPayload>> {
  let session_context = SessionContext::from_context(ctx)?;

  match &session_context.payload {
    Some(payload) => Ok(Some(payload.clone().into())),
    None => Ok(None),
  }
}
```

### Example 2: Session Creation (`src/graphql/resolvers/create_session.rs`)

Note: This resolver doesn't require authentication but creates sessions:

```rust
async fn create_session(
  &self,
  ctx: &Context<'_>,
  input: CreateSessionInput,
) -> Result<CreateSessionResponse> {
  let pool = ctx.data::<SqlitePool>()?;
  let config = ctx.data::<Arc<DpsAuthApiConfig>>()?;
  
  // Session creation logic...
  // Note: No session extraction needed for sign-in
}
```

### Example 3: User Creation (`src/graphql/resolvers/create_user.rs`)

Note: This resolver also doesn't require authentication:

```rust
async fn create_user(
  &self,
  ctx: &Context<'_>,
  input: CreateUserInput,
) -> Result<CreateUserResponse> {
  let pool = ctx.data::<SqlitePool>()?;
  
  // User creation logic...
  // Note: No session extraction needed for user registration
}
```

## Common Patterns and Best Practices

### 1. Error Handling

Always handle session context extraction errors gracefully:

```rust
// Good: Handle missing context
let session_context = match SessionContext::from_context(ctx) {
  Ok(ctx) => ctx,
  Err(e) => {
    tracing::error!("Session context extraction failed: {}", e);
    return Err("Internal server error".into());
  }
};

// Better: Use the ? operator (context includes error logging)
let session_context = SessionContext::from_context(ctx)?;
```

### 2. Authentication Checks

Use consistent patterns for authentication:

```rust
// Pattern 1: Early return
if !session_context.authenticated() {
  return Err("Authentication required".into());
}

// Pattern 2: Match on user_id
match session_context.user_id() {
  Some(user_id) => {
    // Authenticated logic
  }
  None => {
    return Err("Authentication required".into());
  }
}
```

### 3. Database Integration

Combine session data with database operations:

```rust
async fn get_user_profile(&self, ctx: &Context<'_>) -> Result<UserProfile> {
  let session_context = SessionContext::from_context(ctx)?;
  let pool = ctx.data::<SqlitePool>()?;
  
  let user_id = session_context.user_id()
    .ok_or("Authentication required")?;
  
  // Use user_id in database query
  let user = sqlx::query_as!(
    UserProfile,
    "SELECT id, username, created_ts FROM users WHERE id = ?",
    user_id
  )
  .fetch_one(pool)
  .await
  .map_err(|e| format!("Database error: {}", e))?;
  
  Ok(user)
}
```

### 4. Context Data Extraction

Extract other context data alongside session:

```rust
async fn complex_operation(&self, ctx: &Context<'_>) -> Result<MyResponse> {
  // Extract session
  let session_context = SessionContext::from_context(ctx)?;
  
  // Extract database pool
  let pool = ctx.data::<SqlitePool>()?;
  
  // Extract config
  let config = ctx.data::<Arc<DpsAuthApiConfig>>()?;
  
  // Use all context data
  let user_id = session_context.user_id()
    .ok_or("Authentication required")?;
  
  MyService::perform_operation(user_id, pool, &config).await
    .map_err(|e| e.into())
}
```

## Error Handling Approaches

### 1. Authentication Errors

```rust
// Standard authentication error
return Err("Authentication required".into());

// More descriptive error
return Err("You must be logged in to perform this action".into());

// Error with extension data
let mut error = async_graphql::Error::new("Authentication required");
error = error.extend_with(|_, e| e.set("code", "AUTH_REQUIRED"));
return Err(error);
```

### 2. Authorization Errors

```rust
// After authentication, check permissions
if !user_has_permission(user_id, required_permission) {
  return Err("Insufficient permissions".into());
}
```

### 3. Service Error Mapping

Map service errors to user-friendly messages:

```rust
match MyService::do_something(user_id).await {
  Ok(result) => Ok(result),
  Err(ServiceError::NotFound) => Err("Resource not found".into()),
  Err(ServiceError::PermissionDenied) => Err("Access denied".into()),
  Err(ServiceError::DatabaseError(e)) => {
    tracing::error!("Database error: {}", e);
    Err("Internal server error".into())
  }
}
```

## Testing Strategies

### 1. Unit Tests with Mock Context

```rust
#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use async_graphql::*;
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[tokio::test]
  async fn test_authenticated_resolver() {
    let resolver = MyAuthenticatedResolver;
    
    // Create authenticated session
    let payload = ServiceSessionPayload {
      sub: 123,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(payload));

    let schema = Schema::build(resolver, EmptyMutation, EmptySubscription)
      .data(session_context)
      .finish();

    let result = schema.execute("{ protectedOperation }").await;
    
    assert!(result.errors.is_empty());
    // Verify response contains expected data
  }

  #[tokio::test]
  async fn test_unauthenticated_resolver() {
    let resolver = MyAuthenticatedResolver;
    let session_context = SessionContext::new(None);

    let schema = Schema::build(resolver, EmptyMutation, EmptySubscription)
      .data(session_context)
      .finish();

    let result = schema.execute("{ protectedOperation }").await;
    
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }
}
```

### 2. Integration Tests

```rust
#[tokio::test]
async fn test_resolver_with_database() {
  let (pool, _temp_file) = create_test_database().await;
  
  // Setup test data
  setup_test_user(&pool).await;
  
  let resolver = MyResolver;
  let session_context = create_test_session_context(123);
  
  let schema = Schema::build(resolver, EmptyMutation, EmptySubscription)
    .data(pool)
    .data(session_context)
    .finish();

  let result = schema.execute("{ userSpecificData }").await;
  
  assert!(result.errors.is_empty());
  // Verify database integration works
}
```

### 3. Context Missing Tests

```rust
#[tokio::test]
async fn test_missing_session_context() {
  let resolver = MyResolver;
  
  // Create schema WITHOUT session context
  let schema = Schema::build(resolver, EmptyMutation, EmptySubscription).finish();
  
  let result = schema.execute("{ someOperation }").await;
  
  assert!(!result.errors.is_empty());
  assert!(result.errors[0].message.contains("Internal server error"));
}
```

## Quick Reference

### Essential Imports

```rust
use crate::middleware::session::SessionContext;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool; // if using database
use std::sync::Arc; // if using config
```

### Session Extraction Pattern

```rust
// Always start with this
let session_context = SessionContext::from_context(ctx)?;

// Then use one of these patterns:
if !session_context.authenticated() {
  return Err("Authentication required".into());
}

let user_id = session_context.user_id()
  .ok_or("Authentication required")?;
```

### Context Data Extraction

```rust
let pool = ctx.data::<SqlitePool>()?;
let config = ctx.data::<Arc<DpsAuthApiConfig>>()?;
```

### Error Patterns

```rust
// Authentication error
Err("Authentication required".into())

// Service error with logging
tracing::error!("Operation failed: {}", e);
Err("Internal server error".into())

// Detailed error
Err(format!("Operation failed: {}", e).into())
```

This guide provides the essential patterns for working with session data in GraphQL resolvers. Follow these examples to ensure consistent, secure, and maintainable authentication handling across your resolvers.