# Plan: Implement Logout Mutation

**Date**: 2025-11-23@10:50  
**Task**: Add a logout mutation that removes the session cookie from the response

## Overview
Implement a simple logout mutation that clears the session cookie by setting it with an expired date in the past. This follows standard web security practices for cookie-based session termination.

## Files to Create/Modify

### 1. Create: `src/graphql/resolvers/auth_logout.rs`
Create a new resolver file for the logout mutation with the following structure:

```rust
use crate::middleware::session::SESSION_COOKIE_NAME;
use crate::DpsAuthApiConfig;
use async_graphql::{Context, Object, Result, SimpleObject};
use std::sync::Arc;
use tracing::instrument;

/// GraphQL output type for logout response
#[derive(SimpleObject)]
pub struct AuthLogoutResponse {
  /// Success message
  pub message: String,
}

/// GraphQL mutation for user logout
#[derive(Default)]
pub struct AuthLogoutResolver;

#[Object]
impl AuthLogoutResolver {
  /// Logout user by clearing the session cookie
  #[instrument(skip(self, ctx))]
  async fn auth_logout(&self, ctx: &Context<'_>) -> Result<AuthLogoutResponse> {
    let config = ctx.data::<Arc<DpsAuthApiConfig>>()?;
    let cookie_domain = config.cookie_domain.clone();
    let insecure_cookie = config.insecure_cookie;

    // Set cookie to expire in the past to effectively delete it
    let cookie_value = format!(
      "{}=; Domain={}; Path=/; HttpOnly; SameSite=Strict{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
      SESSION_COOKIE_NAME,
      cookie_domain,
      if insecure_cookie { "" } else { "; Secure" }
    );

    // Use append to set the expired cookie
    let _ = ctx.append_http_header("set-cookie", cookie_value);

    Ok(AuthLogoutResponse {
      message: "Successfully logged out".to_string(),
    })
  }
}
```

### 2. Modify: `src/graphql/resolvers/mod.rs`
Add the new resolver to the module exports:

```rust
// Add this line with other pub mod declarations:
pub mod auth_logout;

// Add this line with other pub use declarations:
pub use auth_logout::AuthLogoutResolver;
```

### 3. Modify: `src/graphql/schema.rs`
Add the logout resolver to the Mutation struct:

```rust
// Update the imports to include AuthLogoutResolver:
use crate::graphql::resolvers::{
  AddSiteResolver, AuthChangePasswordResolver, AuthLoginResolver, AuthLogoutResolver, AuthMeResolver,
  AuthRegisterResolver, GetServerTimestampResolver, RemoveSiteResolver, SitesResolver,
  UpdateSiteResolver,
};

// Update the Mutation struct documentation to include logout:
/// Available mutations:
/// - authRegister: Register a new user account
/// - authLogin: Authenticate user and create session
/// - authLogout: Logout user by clearing session cookie
/// - authChangePassword: Change password for authenticated user
/// - addSite: Add a new site to the database (requires can_create_site permission)
/// - updateSite: Update an existing site (requires can_update_site permission)
/// - removeSite: Remove an existing site (requires can_delete_site permission)

// Update the Mutation struct definition:
#[derive(MergedObject, Default)]
pub struct Mutation(
  AuthRegisterResolver,
  AuthLoginResolver,
  AuthLogoutResolver,
  AuthChangePasswordResolver,
  AddSiteResolver,
  RemoveSiteResolver,
  UpdateSiteResolver,
);

// Update the Mutation::new() method:
impl Mutation {
  pub fn new() -> Self {
    Self(
      AuthRegisterResolver,
      AuthLoginResolver,
      AuthLogoutResolver,
      AuthChangePasswordResolver,
      AddSiteResolver,
      RemoveSiteResolver,
      UpdateSiteResolver,
    )
  }
}
```

## Implementation Details

### Cookie Deletion Strategy
The logout mutation will set the session cookie with:
- Same name (`DpsAuthSession`)
- Same domain and path settings
- `Expires=Thu, 01 Jan 1970 00:00:00 GMT` (Unix epoch)
- Same security flags (Secure/HttpOnly/SameSite)

This ensures the browser immediately deletes the cookie.

### GraphQL Schema
The mutation will be available as:
```graphql
mutation {
  authLogout {
    message
  }
}
```

### Security Considerations
- No authentication required for logout (safe operation)
- Uses same cookie domain and security settings as login
- Follows standard web practices for cookie deletion
- No server-side session storage cleanup needed (stateless JWT tokens)

### Testing
The resolver should include basic tests to verify:
- Cookie header is set correctly
- Response message is returned
- Cookie has correct expiration date

## Dependencies
No new dependencies required. Uses existing:
- `async_graphql` for GraphQL integration
- `tracing` for instrumentation
- Existing config and middleware constants

### 4. Modify: `README.md`
Update the mutations table in the README.md to include both the new `authLogout` mutation and the existing `authChangePassword` mutation. Add the following rows to the mutations table:

```markdown
| `authLogout` | Logout user by clearing session cookie | None | message (String) | Valid session cookie |
| `authChangePassword` | Change password for authenticated user | currentPassword (String!), newPassword (String!) | message (String) | Valid session cookie |
```

Insert these rows after the `authLogin` row in the existing mutations table.

## Backwards Compatibility
This is a new feature with no breaking changes to existing functionality.