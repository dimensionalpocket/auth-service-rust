# Convert Resolvers to Use Orchestrator Pattern

## Date Created
2025-12-30@22:00

## Problem Statement
Several GraphQL resolvers in the codebase are calling service methods directly instead of orchestrator methods, which is inconsistent with the project's architectural pattern. The project follows the structure:
- **Resolvers** handle GraphQL-specific concerns (input validation, response formatting)
- **Orchestrators** handle the pattern: authentication → authorization → business logic (calling services)
- **Services** contain core business logic and interact with the database

## Full Report: Inconsistent Resolvers

### Resolvers Calling Services Directly (Need Conversion)

| File | GraphQL Operation | Currently Calls | Should Call | Required Orchestrator Method |
|------|-------------------|----------------|--------------|---------------------------|
| `auth_login.rs` | `authLogin` mutation | `AuthService::login()` | `AuthOrchestrator::login_user()` | `login_user()` |
| `auth_register.rs` | `authRegister` mutation | `AuthService::register()` | `AuthOrchestrator::register_user()` | `register_user()` |
| `auth_me.rs` | `authMe` query | `AuthService::get_current_user()` | `AuthOrchestrator::get_authenticated_user()` | `get_authenticated_user()` |
| `sites.rs` | `sites` query | `SiteService::get_all_sites()` | (Keep as-is, see below) | N/A |

### Resolvers Already Following the Pattern (No Changes Needed)

These resolvers already call orchestrator methods correctly:
- `auth_change_password.rs` → `AuthOrchestrator::change_authenticated_user_password()`
- `user.rs` → `UserOrchestrator::get_user_details_with_permission_check()`
- `users.rs` → `UserOrchestrator::list_users_with_permission_check()`
- `role.rs` → `RoleOrchestrator::get_role_by_id_with_permission_check()`
- `roles.rs` → `RoleOrchestrator::get_all_roles_with_permission_check()`
- `add_role.rs` → `RoleOrchestrator::create_role_with_permission_check()`
- `delete_user.rs` → `UserOrchestrator::delete_user_with_permission_check()`
- `remove_role.rs` → `RoleOrchestrator::delete_role_with_permission_check()`
- `update_role.rs` → `RoleOrchestrator::update_role_with_permission_check()`
- `role_permissions.rs` → `RoleOrchestrator::get_all_role_permissions_with_permission_check()`
- `set_default_role.rs` → `RoleOrchestrator::set_default_role_with_permission_check()`
- `site.rs` → `SiteOrchestrator::get_site_details_with_permission_check()`
- `add_site.rs` → `SiteOrchestrator::create_site_with_permission_check()`
- `remove_site.rs` → `SiteOrchestrator::remove_site_with_permission_check()`
- `update_site.rs` → `SiteOrchestrator::update_site_with_permission_check()`

### Special Cases

| File | GraphQL Operation | Current Behavior | Recommendation |
|------|-------------------|------------------|----------------|
| `auth_logout.rs` | `authLogout` mutation | Sets cookie to expire | **Keep as-is** (see special case analysis below) |
| `server_timestamp.rs` | `serverTimestamp` query | Calls `ServerService::get_server_timestamp()` | **Keep as-is** - simple utility without auth/authz |
| `sites.rs` | `sites` query | Calls `SiteService::get_all_sites()` | **Keep as-is** - public query without auth/authz |

---

## Special Case Analysis: Logout Resolver

### Current Implementation
The `auth_logout.rs` resolver (lines 1-242) only handles cookie deletion:

```rust
async fn auth_logout(&self, ctx: &Context<'_>) -> Result<AuthLogoutResponse> {
  let config = ctx.data::<DpsAuthApiConfig>()?;
  let cookie_domain = config.cookie_domain.clone();
  let insecure_cookie = config.insecure_cookie;

  // Set cookie to expire in the past to effectively delete it
  let cookie_value = format!(
    "{}=; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
    SESSION_COOKIE_NAME,
    cookie_domain,
    config.api_path,
    if insecure_cookie { "" } else { "; Secure" }
  );

  // Use append to set the expired cookie
  let _ = ctx.append_http_header("set-cookie", cookie_value);

  Ok(AuthLogoutResponse {
    message: "Successfully logged out".to_string(),
  })
}
```

### Challenge
The logout operation is fundamentally different from other operations:
- No database interaction required
- No authentication/authorization check needed (user can logout anytime)
- Purely a framework-level concern (HTTP response headers)

### Recommendation: Keep as a Special Case

**Rationale:**

1. **No Business Logic**: Logout doesn't require authentication verification, authorization checks, or business logic. It's purely a transport-level concern (removing the session cookie).

2. **No Orchestrator Pattern Match**: The orchestrator pattern is designed for:
   - Authentication → Authorization → Business Logic
   - Logout doesn't fit this pattern (it has no authz, no service calls, no database access)

3. **Framework Concern**: Cookie manipulation is a GraphQL framework/HTTP concern. `ctx.append_http_header()` is specific to async-graphql's response handling.

4. **No Testability Loss**: The resolver already has comprehensive tests covering all cookie configurations (secure, insecure, domain, path).

5. **Existing Precedent**: `server_timestamp.rs` and `sites.rs` are also simple resolvers that call services directly without orchestrators because they don't need the auth/authz pattern.

**Conclusion**: The logout resolver should remain as-is. Creating an orchestrator for logout would introduce unnecessary complexity without any benefit. The resolver's single responsibility is to tell the HTTP layer to delete the cookie, which is exactly what it does.

---

## Implementation Plan

### Phase 1: Add AuthOrchestrator Methods
**File**: `src/orchestrators/auth_orchestrator.rs`

Add three new public async methods:

#### 1. `login_user()`
```rust
pub async fn login_user(
  pool: &SqlitePool,
  username: &str,
  password: &str,
  session_secret: &[u8],
) -> Result<AuthLoginResult, SessionError>
```

**Logic**:
- Call `AuthService::login()` directly (no auth/authz needed for login)
- Return the result (session token + user info)

**Tests needed**:
- Success case
- Invalid credentials
- Database error

#### 2. `register_user()`
```rust
pub async fn register_user(
  pool: &SqlitePool,
  username: &str,
  password: &str,
  password_confirmation: &str,
  session_secret: &[u8],
) -> Result<AuthRegisterResult, UserError>
```

**Logic**:
- Call `AuthService::register()` directly (no auth/authz needed for registration)
- Return the result (session token + created user info)

**Tests needed**:
- Success case
- Duplicate username
- Password mismatch
- Validation errors

#### 3. `get_authenticated_user()`
```rust
pub async fn get_authenticated_user(
  pool: &SqlitePool,
  session_context: SessionContext,
) -> Result<Option<AuthMeResult>, SessionError>
```

**Logic**:
- Check if session_context has a payload
- If None, return `Ok(None)`
- If Some, call `AuthService::get_current_user()` with the session context
- Return user info or None

**Tests needed**:
- Authenticated user returns data
- Unauthenticated (no payload) returns None
- Database error handling

### Phase 2: Update `auth_login.rs` Resolver
**File**: `src/graphql/resolvers/auth_login.rs`

**Changes**:
1. Update imports to include `AuthOrchestrator`
2. Replace `AuthService::login()` call with `AuthOrchestrator::login_user()`
3. Remove the `map_session_error_to_user_message()` helper function (error mapping will be done by orchestrator)

**Updated resolver logic**:
```rust
match AuthOrchestrator::login_user(
  &mut conn,
  &username,
  &password,
  &session_secret
).await {
  Ok(auth_result) => {
    // Set session cookie (this stays in resolver)
    let cookie_value = format!(
      "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
      SESSION_COOKIE_NAME,
      auth_result.session_token,
      cookie_domain,
      config.api_path,
      if insecure_cookie { "" } else { "; Secure" },
      config.session_ttl_seconds
    );

    let _ = ctx.append_http_header("set-cookie", cookie_value);

    Ok(AuthLoginResponse {
      token: auth_result.session_token,
      user: UserWithRoleResponse { /* ... */ },
      message: "Authentication successful".to_string(),
    })
  }
  Err(session_error) => {
    // Map SessionError to GraphQL errors
    Err(async_graphql::Error::new(session_error.to_user_message()))
  }
}
```

**Tests to update**:
- All existing tests should pass with minimal changes
- Ensure error messages match

### Phase 3: Update `auth_register.rs` Resolver
**File**: `src/graphql/resolvers/auth_register.rs`

**Changes**:
1. Update imports to include `AuthOrchestrator`
2. Replace `AuthService::register()` call with `AuthOrchestrator::register_user()`

**Updated resolver logic**:
```rust
match AuthOrchestrator::register_user(
  &mut conn,
  &username,
  &password,
  &password_confirmation,
  &session_secret,
).await {
  Ok(register_result) => {
    // Set session cookie (this stays in resolver)
    let cookie_value = format!(/* ... */);
    let _ = ctx.append_http_header("set-cookie", cookie_value);

    Ok(AuthRegisterResponse { /* ... */ })
  }
  Err(UserError::UsernameAlreadyExists(username)) => {
    Err(async_graphql::Error::new(format!(
      "Username '{username}' is already in use"
    )))
  }
  // ... other error mappings
}
```

**Tests to update**:
- All existing tests should pass with minimal changes

### Phase 4: Update `auth_me.rs` Resolver
**File**: `src/graphql/resolvers/auth_me.rs`

**Changes**:
1. Update imports to include `AuthOrchestrator`
2. Replace `AuthService::get_current_user()` call with `AuthOrchestrator::get_authenticated_user()`

**Updated resolver logic**:
```rust
match AuthOrchestrator::get_authenticated_user(pool, session_context).await {
  Ok(Some(auth_me_result)) => Ok(Some(AuthMeResponse {
    user_id: auth_me_result.user_id,
    uuid: auth_me_result.uuid,
    // ...
  })),
  Ok(None) => Ok(None), // Unauthenticated
  Err(session_error) => {
    let user_message = map_session_error_to_user_message(&session_error);
    Err(async_graphql::Error::new(user_message))
  }
}
```

**Tests to update**:
- Test authenticated user returns data
- Test unauthenticated returns None
- Test error handling

### Phase 5: Update Documentation
**File**: `README.md`

After all implementation and testing is complete, update the mutations/queries table in README.md to reflect any changes to the GraphQL schema structure.

---

## Testing Strategy

For each phase:
1. Run existing tests: `cargo test --quiet`
2. Verify all tests pass
3. Check for any test failures due to changed error messages or behavior
4. Update tests if needed to match new orchestrator behavior

### Final Verification
After completing all phases:
1. Run full test suite: `cargo test --quiet`
2. Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`
3. Manually test GraphQL operations:
   - Login
   - Register
   - Auth me (authenticated and unauthenticated)
   - Logout

---

## Summary

| Phase | Description | Files Modified |
|-------|-------------|----------------|
| 1 | Add orchestrator methods for login, register, and auth_me | `src/orchestrators/auth_orchestrator.rs` |
| 2 | Update auth_login resolver to use orchestrator | `src/graphql/resolvers/auth_login.rs` |
| 3 | Update auth_register resolver to use orchestrator | `src/graphql/resolvers/auth_register.rs` |
| 4 | Update auth_me resolver to use orchestrator | `src/graphql/resolvers/auth_me.rs` |
| 5 | Update README.md table | `README.md` |

### Resolvers Requiring Changes
- `auth_login.rs` (Phase 2)
- `auth_register.rs` (Phase 3)
- `auth_me.rs` (Phase 4)

### Resolvers Keeping Current Implementation
- `auth_logout.rs` - Special case, cookie deletion is a framework concern
- `server_timestamp.rs` - Simple utility, no auth/authz needed
- `sites.rs` - Public query, no auth/authz needed
