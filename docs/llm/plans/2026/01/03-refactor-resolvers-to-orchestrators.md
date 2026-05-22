# Plan: Resolvers Not Calling Orchestrators

**Date**: 2026-01-03@20:35
**Status**: Completed

## Summary

This plan addresses GraphQL resolvers that are not following the project's architecture pattern of calling orchestrators. Based on the architectural guidelines, resolvers should be thin wrappers that call **at most** a single service or orchestrator, with orchestrators being the preferred approach for authentication, authorization, and business logic workflows.

## Report: Resolvers Not Using Orchestrators

| Resolver | File | Current Pattern | Recommendation |
|----------|-------|-----------------|----------------|
| `serverTimestamp` | `server_timestamp.rs` | Calls `ServerService::get_server_timestamp()` directly | **Keep as-is** - Simple utility query with no authentication/authorization/database access |
| `sites` | `sites.rs` | Calls `SiteService::get_all_sites()` directly, acquires connection from pool | **Create orchestrator** - Add `GetSitesOrchestrator` for consistency with other list queries |
| `authLogin` | `auth_login.rs` | Calls `AuthService::login()` and `CookieService` directly, acquires connection | **Create orchestrator** - Add `AuthLoginOrchestrator` that returns `(AuthResult, cookie_value)` |
| `authRegister` | `auth_register.rs` | Calls `AuthService::register()` and `CookieService` directly, acquires connection | **Create orchestrator** - Add `AuthRegisterOrchestrator` that returns `(AuthResult, cookie_value)` |
| `authLogout` | `auth_logout.rs` | Calls `CookieService` directly | **Keep as-is** - Cookie-only operation, no business logic needed |

## Case-by-Case Implementation Plan

---

### Case 1: `sites` Resolver

**Current Implementation**: `sites.rs` (lines 43-62)
- Acquires database connection from pool
- Calls `SiteService::get_all_sites(&mut conn)` directly
- Maps to `SiteListing` response type

**Analysis**:
- This is inconsistent with `users.rs` and `roles.rs` which both use orchestrators (`GetUsersOrchestrator`, `GetRolesOrchestrator`)
- The resolver performs no authentication/authorization (public query)
- No orchestrator exists for this query

**Recommendation**: Create `GetSitesOrchestrator`

**Implementation**:

#### Phase 1: Create `GetSitesOrchestrator`
- File: `src/orchestrators/site/get_sites.rs`
- Since no authentication is required, the orchestrator will simply delegate to `SiteService::get_all_sites()`
- Return type: `Vec<Site>` or similar model
- No session context needed

**Files to Create**:
- `src/orchestrators/site/get_sites.rs`

**Code Sample for `get_sites.rs`**:
```rust
use crate::models::Site;
use crate::services::SiteService;
use crate::types::SiteError;
use sqlx::SqlitePool;

pub struct GetSitesOrchestrator;

impl GetSitesOrchestrator {
  pub async fn run(pool: &SqlitePool) -> Result<Vec<Site>, SiteError> {
    let mut conn = pool.acquire().await.map_err(SiteError::DatabaseError)?;

    SiteService::get_all_sites(&mut conn).await
  }
}
```

#### Phase 2: Update `sites.rs` resolver
- Remove direct service call and connection acquisition
- Call `GetSitesOrchestrator::run(pool)`
- Keep existing response mapping logic

**Files to Modify**:
- `src/graphql/resolvers/sites.rs` (lines 43-62)

#### Phase 3: Update orchestrator module exports
- Add `pub mod get_sites;` to `src/orchestrators/site/mod.rs`

**Files to Modify**:
- `src/orchestrators/site/mod.rs`

#### Phase 4: Create tests
- Move existing tests to continue working
- No behavior changes expected

---

### Case 2: `authLogin` Resolver

**Current Implementation**: `auth_login.rs` (lines 29-73)
- Acquires database connection from pool
- Calls `AuthService::login(&mut conn, &username, &password, &config.session_secret)`
- Generates session cookie via `CookieService::generate_session_cookie()`
- Sets cookie header via `ctx.append_http_header()`

**Analysis**:
- Complex authentication workflow involving:
  - Database access (user lookup)
  - Password verification
  - Session token generation
  - Cookie management
- Would benefit from orchestrator pattern for consistency

**Recommendation**: Create `AuthLoginOrchestrator`

**Implementation**:

#### Phase 1: Create `AuthLoginOrchestrator`
- File: `src/orchestrators/auth/auth_login.rs`
- The orchestrator should:
  - Call `AuthService::login()` to get auth result
  - Call `CookieService::generate_session_cookie()` to generate cookie
  - Return both the auth result AND the cookie value as a tuple
- Orchestrator should NOT set response headers (that's resolver's job)

**Considerations**:
- The orchestrator should accept `config` (which contains `session_secret` for token generation and cookie settings)
- Return type: `(AuthService::AuthResult, String)` where String is the cookie value
- The resolver will use the returned cookie value to set the header via `ctx.append_http_header()`

**Files to Create**:
- `src/orchestrators/auth/auth_login.rs`

**Code Sample for `auth_login.rs` (orchestrator)**:
```rust
use crate::services::{AuthService, CookieService};
use crate::types::SessionError;
use crate::DpsAuthApiConfig;
use sqlx::{Pool, Sqlite};

pub struct AuthLoginOrchestrator;

impl AuthLoginOrchestrator {
    pub async fn run(
        pool: &Pool<Sqlite>,
        username: &str,
        password: &str,
        config: &DpsAuthApiConfig,
    ) -> Result<(AuthService::AuthResult, String), SessionError> {
        let mut conn = pool.acquire().await.map_err(SessionError::DatabaseError)?;

        let auth_result = AuthService::login(&mut conn, username, password, &config.session_secret).await?;

        let cookie_value = CookieService::generate_session_cookie(config, &auth_result.session_token);

        Ok((auth_result, cookie_value))
    }
}
```

#### Phase 2: Update `auth_login.rs` resolver
- Remove direct `AuthService::login()` call
- Remove `CookieService` import and call
- Remove connection acquisition
- Call `AuthLoginOrchestrator::run(pool, &username, &password, &config)`
- Set cookie header using returned `cookie_value` via `ctx.append_http_header()`
- Keep response mapping logic

**Files to Modify**:
- `src/graphql/resolvers/auth_login.rs` (lines 29-73)

#### Phase 3: Update orchestrator module exports
- Add `pub mod auth_login;` to `src/orchestrators/auth/mod.rs`

**Files to Modify**:
- `src/orchestrators/auth/mod.rs`

#### Phase 4: Ensure tests pass
- All existing tests should continue to pass
- No behavior changes expected

---

### Case 3: `authRegister` Resolver

**Current Implementation**: `auth_register.rs` (lines 50-108)
- Acquires database connection from pool
- Calls `AuthService::register(&mut conn, &username, &password, &password_confirmation, &config.session_secret)`
- Generates session cookie via `CookieService::generate_session_cookie()`
- Sets cookie header via `ctx.append_http_header()`

**Analysis**:
- Similar complexity to `authLogin`
- Workflow includes:
  - Input validation
  - Username uniqueness check
  - Password validation
  - Password hashing
  - User creation
  - Session token generation
  - Default role assignment
- Would benefit from orchestrator pattern

**Recommendation**: Create `AuthRegisterOrchestrator`

**Implementation**:

#### Phase 1: Create `AuthRegisterOrchestrator`
- File: `src/orchestrators/auth/auth_register.rs`
- The orchestrator should:
  - Call `AuthService::register()` to get auth result
  - Call `CookieService::generate_session_cookie()` to generate cookie
  - Return both the auth result AND the cookie value as a tuple
- Orchestrator should NOT set response headers (that's resolver's job)

**Considerations**:
- The orchestrator should accept `config` (which contains `session_secret` for token generation and cookie settings)
- Return type: `(AuthService::AuthResult, String)` where String is the cookie value
- Use existing `AuthService::AuthResult` type (do NOT create a new type)
- The resolver will use the returned cookie value to set the header via `ctx.append_http_header()`

**Files to Create**:
- `src/orchestrators/auth/auth_register.rs`

**Code Sample for `auth_register.rs` (orchestrator)**:
```rust
use crate::services::{AuthService, CookieService};
use crate::types::SessionError;
use crate::DpsAuthApiConfig;
use sqlx::{Pool, Sqlite};

pub struct AuthRegisterOrchestrator;

impl AuthRegisterOrchestrator {
    pub async fn run(
        pool: &Pool<Sqlite>,
        username: &str,
        password: &str,
        password_confirmation: &str,
        config: &DpsAuthApiConfig,
    ) -> Result<(AuthService::RegisterResult, String), SessionError> {
        let mut conn = pool.acquire().await.map_err(SessionError::DatabaseError)?;

        let auth_result = AuthService::register(&mut conn, username, password, password_confirmation, &config.session_secret)
            .map_err(|e| SessionError::AuthenticationError(e.to_string()))?;

        let cookie_value = CookieService::generate_session_cookie(config, &auth_result.session_token);

        Ok((auth_result, cookie_value))
    }
}
```

#### Phase 2: Update `auth_register.rs` resolver
- Remove direct `AuthService::register()` call
- Remove `CookieService` import and call
- Remove connection acquisition
- Call `AuthRegisterOrchestrator::run(pool, &username, &password, &password_confirmation, &config)`
- Set cookie header using returned `cookie_value` via `ctx.append_http_header()`
- Keep response mapping logic

**Files to Modify**:
- `src/graphql/resolvers/auth_register.rs` (lines 50-108)

#### Phase 3: Update orchestrator module exports
- Add `pub mod auth_register;` to `src/orchestrators/auth/mod.rs`

**Files to Modify**:
- `src/orchestrators/auth/mod.rs`

#### Phase 4: Ensure tests pass
- All existing tests should continue to pass
- No behavior changes expected

---

### Case 4: `authLogout` Resolver

**Current Implementation**: `auth_logout.rs` (lines 22-34)
- Gets config from context
- Calls `CookieService::generate_logout_cookie(config)`
- Sets cookie header via `ctx.append_http_header()`

**Analysis**:
- Pure cookie manipulation operation
- No database access
- No authentication/authorization logic
- No business logic

**Recommendation**: **No change needed**

**Reasoning**:
- This is a simple HTTP header manipulation
- No orchestration, authentication, or authorization is needed
- Directly calling `CookieService` is appropriate
- Creating an orchestrator would add unnecessary complexity

**Files to Modify**:
- None

---

### Case 5: `serverTimestamp` Resolver

**Current Implementation**: `server_timestamp.rs` (lines 26-28)
- Calls `ServerService::get_server_timestamp()` directly
- Returns timestamp as string

**Analysis**:
- Pure utility function
- No database access
- No authentication/authorization
- Returns static server time

**Recommendation**: **No change needed**

**Reasoning**:
- Simple utility query
- No business logic, authentication, or authorization
- Direct service call is appropriate and simple
- Creating an orchestrator would add unnecessary complexity

**Files to Modify**:
- None

---

## Testing Strategy

For each case requiring implementation:

1. **Run existing tests** to ensure no regression
2. **Add integration tests** for new orchestrators
3. **Verify resolver behavior** unchanged from user perspective
4. **Run linter**: `cargo clippy --allow-dirty --fix && cargo fmt`

---

## Implementation Order

Recommended implementation order (safest to most complex):

1. **`sites`** (Case 1) - Simplest, public query, minimal refactoring
2. **`authRegister`** (Case 3) - Similar to authLogin but tested independently
3. **`authLogin`** (Case 2) - Most complex, last to test

---

## Notes

- **`sites` vs `users`/`roles`**: All three are list queries, but only `sites` lacks an orchestrator. Adding one makes the codebase more consistent.
- **Auth orchestrators**: Both `authLogin` and `authRegister` have similar patterns. Creating orchestrators for both provides consistency in the auth domain.
- **Cookie management**: For `authLogin` and `authRegister`, the orchestrator will generate the cookie value using `CookieService` and return it to the resolver. The resolver then uses the returned cookie value to set the HTTP header via `ctx.append_http_header()`. This approach:
  - Keeps cookie generation in the business logic layer (orchestrator)
  - Keeps HTTP header setting in the GraphQL/resolver layer (appropriate separation of concerns)
  - Ensures orchestrators handle all business logic, including cookie generation
  - Ensures resolvers remain as thin wrappers focused on GraphQL-specific concerns
- **No backwards compatibility concerns**: The project is pre-1.0.0, so breaking changes to internal architecture are acceptable.

---

## Final Checklist

After implementation is complete:

- [ ] All new orchestrators are created
- [ ] All target resolvers updated to use orchestrators
- [ ] Module exports updated in `src/orchestrators/*/mod.rs`
- [ ] All existing tests pass
- [ ] Linter runs without errors
- [ ] Code follows existing patterns and conventions
