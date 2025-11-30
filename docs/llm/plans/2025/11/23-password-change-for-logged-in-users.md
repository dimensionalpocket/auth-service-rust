# Password Change for Logged-in Users Implementation Plan

## Overview

This plan outlines the implementation of password changing functionality for authenticated users in the DPS Auth API system. The feature will allow logged-in users to change their password through a GraphQL mutation while maintaining security best practices.

## Security Analysis: Current Password Requirement

### Decision: **Require current password**

**Rationale:**
1. **Prevents account takeover** - If a session is compromised or left open on a public device, requiring the current password prevents unauthorized password changes
2. **Defense in depth** - Even with session authentication, verifying current password provides an additional security layer
3. **Industry standard** - Most secure systems require current password for password changes (Google, GitHub, banks, etc.)
4. **Session hijacking protection** - Prevents attackers with stolen session cookies from locking out legitimate users

**Security considerations:**
- Rate limiting should be applied to prevent brute force attacks
- Failed attempts should be logged but not reveal whether username exists
- Current password verification should use the same secure Argon2 verification as login

## Implementation Details

### 1. Database Layer Changes

#### File: `src/queries/users/update_user_password.rs` (NEW)

```rust
use sqlx::SqlitePool;

#[derive(Debug)]
pub struct UpdateUserPasswordData {
  pub user_id: i64,
  pub new_password_hash: String,
}

pub struct UpdateUserPasswordQuery;

impl UpdateUserPasswordQuery {
  pub async fn run(pool: &SqlitePool, data: UpdateUserPasswordData) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
      r#"
      UPDATE users 
      SET password_hash = ?, updated_ts = ?
      WHERE id = ?
      "#,
    )
    .bind(&data.new_password_hash)
    .bind(now)
    .bind(data.user_id)
    .execute(pool)
    .await?;

    Ok(())
  }
}
```

#### File: `src/queries/users/mod.rs` (MODIFY)

Add:
```rust
pub mod update_user_password;
pub use update_user_password::{UpdateUserPasswordData, UpdateUserPasswordQuery};
```

### 2. Service Layer Implementation

#### File: `src/services/user_service.rs` (MODIFY)

Add new error variant to `UserError` enum:
```rust
/// Current password verification failed
InvalidCurrentPassword,
```

Add new method to `UserService`:
```rust
/// Update user password with current password verification
///
/// This method:
/// 1. Validates the new password meets requirements
/// 2. Verifies the current password is correct
/// 3. Hashes the new password using Argon2
/// 4. Updates the password in the database
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - ID of the user whose password to update
/// * `current_password` - Current password for verification
/// * `new_password` - New password to set
///
/// # Returns
/// * `Ok(())` - Password successfully updated
/// * `Err(UserError)` - Update failed due to validation, verification, or database error
#[instrument(skip(pool, current_password, new_password), fields(user_id = %user_id))]
pub async fn update_password(
  pool: &SqlitePool,
  user_id: i64,
  current_password: &str,
  new_password: &str,
) -> Result<(), UserError> {
  // Validate new password
  Self::validate_password(new_password)?;

  // Get current user with password hash
  let user = GetUserByIdQuery::run(pool, user_id)
    .await?
    .ok_or(UserError::UserNotFound(user_id))?;

  // Verify current password using PasswordService
  PasswordService::verify(&user.password_hash, current_password)
    .map_err(|_| UserError::InvalidCurrentPassword)?;

  // Hash new password
  let new_password_hash = PasswordService::generate(new_password)?;

  // Update password in database
  let update_data = UpdateUserPasswordData {
    user_id,
    new_password_hash,
  };

  UpdateUserPasswordQuery::run(pool, update_data).await?;

  Ok(())
}
```

Add to `Display` implementation:
```rust
UserError::InvalidCurrentPassword => write!(f, "Current password is incorrect"),
```

### 3. GraphQL Resolver Implementation

#### File: `src/graphql/resolvers/auth_change_password.rs` (NEW)

```rust
use crate::middleware::session::SessionContext;
use crate::services::{UserError, UserService};
use async_graphql::{Context, InputObject, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// Input type for password change
#[derive(InputObject)]
pub struct AuthChangePasswordInput {
  /// Current password for verification
  pub current_password: String,
  /// New password to set
  pub new_password: String,
  /// Confirmation of new password to ensure correctness
  pub new_password_confirmation: String,
}

/// GraphQL output type for password change response
#[derive(async_graphql::SimpleObject)]
pub struct AuthChangePasswordResponse {
  /// Success message
  pub message: String,
  /// Timestamp when password was changed
  pub changed_ts: i64,
}

/// GraphQL mutation for changing user password
#[derive(Default)]
pub struct AuthChangePasswordResolver;

#[Object]
impl AuthChangePasswordResolver {
  /// Change the password for the currently authenticated user
  ///
  /// This mutation:
  /// - Requires a valid session (authenticated user)
  /// - Validates the new password and confirmation match
  /// - Verifies the current password is correct
  /// - Updates the password using secure Argon2 hashing
  ///
  /// # Arguments
  /// * `input` - AuthChangePasswordInput containing current password, new password, and confirmation
  ///
  /// # Returns
  /// * `AuthChangePasswordResponse` - Success message and timestamp
  ///
  /// # Errors
  /// * Returns GraphQL error if not authenticated
  /// * Returns GraphQL error if current password is incorrect
  /// * Returns GraphQL error if new password validation fails
  /// * Returns GraphQL error if password and confirmation don't match
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(self, ctx, input))]
  async fn auth_change_password(
    &self,
    ctx: &Context<'_>,
    input: AuthChangePasswordInput,
  ) -> Result<AuthChangePasswordResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    // Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

    // Validate new password confirmation matches
    if input.new_password != input.new_password_confirmation {
      return Err(async_graphql::Error::new(
        "New password and confirmation do not match",
      ));
    }

    match UserService::update_password(
      pool,
      user_id,
      &input.current_password,
      &input.new_password,
    )
    .await
    {
      Ok(()) => Ok(AuthChangePasswordResponse {
        message: "Password changed successfully".to_string(),
        changed_ts: chrono::Utc::now().timestamp(),
      }),
      Err(UserError::InvalidCurrentPassword) => {
        Err(async_graphql::Error::new("Current password is incorrect"))
      }
      Err(UserError::ValidationError(msg)) => {
        Err(async_graphql::Error::new(format!("Validation error: {msg}")))
      }
      Err(err) => {
        tracing::error!("Failed to update password for user {}: {}", user_id, err);
        Err(async_graphql::Error::new("Failed to update password"))
      }
    }
  }
}
```

### 4. Schema Integration

#### File: `src/graphql/resolvers/mod.rs` (MODIFY)

Add:
```rust
pub mod auth_change_password;
pub use auth_change_password::AuthChangePasswordResolver;
```

#### File: `src/graphql/schema.rs` (MODIFY)

Update imports:
```rust
use crate::graphql::resolvers::{
  AddSiteResolver, AuthChangePasswordResolver, AuthLoginResolver, AuthMeResolver, 
  AuthRegisterResolver, GetServerTimestampResolver, RemoveSiteResolver, 
  SitesResolver, UpdateSiteResolver,
};
```

Update Mutation struct:
```rust
#[derive(MergedObject, Default)]
pub struct Mutation(
  AuthRegisterResolver,
  AuthLoginResolver,
  AuthChangePasswordResolver,
  AddSiteResolver,
  RemoveSiteResolver,
  UpdateSiteResolver,
);
```

Update Mutation::new():
```rust
impl Mutation {
  pub fn new() -> Self {
    Self(
      AuthRegisterResolver,
      AuthLoginResolver,
      AuthChangePasswordResolver,
      AddSiteResolver,
      RemoveSiteResolver,
      UpdateSiteResolver,
    )
  }
}
```

### 5. Input Validation Requirements

#### Current Password
- Required field
- Must match existing password hash (verified via Argon2)
- No length validation (handled by password verification)

#### New Password
- Required field
- Must be 6-128 characters long (existing validation)
- Cannot be empty
- Must pass existing password validation rules

#### New Password Confirmation
- Required field
- Must exactly match new_password
- Case-sensitive comparison

### 6. Error Handling Strategy

#### User-Friendly Error Messages
- `"Authentication required"` - No valid session
- `"Current password is incorrect"` - Current password verification failed
- `"New password and confirmation do not match"` - Password confirmation mismatch
- `"Validation error: {specific_message}"` - Input validation failures
- `"Failed to update password"` - Database or system errors

#### Security Considerations
- Log failed attempts with user_id but not password details
- Use generic error messages to avoid information leakage
- Rate limiting should be implemented at the middleware level
- Do not reveal whether user exists in error messages

### 7. Testing Strategy

#### Unit Tests
- Test `UpdateUserPasswordQuery` with valid and invalid user IDs
- Test `UserService::update_password` with:
  - Valid current password and new password
  - Invalid current password
  - Invalid new password (validation failures)
  - Non-existent user

#### Integration Tests
- Test complete GraphQL mutation flow
- Test authentication requirement
- Test password confirmation validation
- Test session context extraction

#### Test Files to Create/Modify
- `src/queries/users/update_user_password.rs` - Add unit tests
- `src/services/user_service.rs` - Add tests for `update_password` method
- `src/graphql/resolvers/auth_change_password.rs` - Add GraphQL resolver tests

### 8. Integration with Existing Patterns

#### Authentication
- Uses existing `SessionContext` from middleware
- Follows same pattern as `AuthMeResolver` for authentication
- Leverages existing session validation

#### Password Handling
- Uses existing `PasswordService` for hashing and verification
- Maintains Argon2 security standards
- Follows existing password validation rules

#### Error Handling
- Follows existing `UserError` enum pattern
- Uses same error mapping approach as other resolvers
- Maintains consistent logging with tracing

#### Database Operations
- Follows existing query pattern with dedicated query struct
- Uses same connection pool approach
- Maintains timestamp update pattern

## Implementation Order

1. **Database Layer** - Create `UpdateUserPasswordQuery`
2. **Service Layer** - Add `update_password` method to `UserService`
3. **GraphQL Resolver** - Create `AuthChangePasswordResolver`
4. **Schema Integration** - Add resolver to GraphQL schema
5. **Testing** - Add comprehensive tests for all layers
6. **Documentation** - Update API documentation

## Security Considerations

1. **Rate Limiting** - Implement rate limiting for password change attempts
2. **Session Invalidation** - Consider invalidating other sessions after password change
3. **Audit Logging** - Log password changes for security auditing
4. **Password Strength** - Maintain existing password validation requirements
5. **Error Messages** - Use generic error messages to prevent information leakage

## Future Enhancements

1. **Password History** - Prevent reuse of recent passwords
2. **Email Notifications** - Notify users of password changes
3. **Multi-Factor Authentication** - Require MFA for password changes
4. **Admin Password Reset** - Allow administrators to reset user passwords
5. **Password Expiry** - Implement password expiration policies

## Files to Modify/Create

### New Files
- `src/queries/users/update_user_password.rs`
- `src/graphql/resolvers/auth_change_password.rs`

### Modified Files
- `src/queries/users/mod.rs`
- `src/services/user_service.rs`
- `src/graphql/resolvers/mod.rs`
- `src/graphql/schema.rs`

This implementation follows the existing codebase patterns while providing secure password change functionality for authenticated users.