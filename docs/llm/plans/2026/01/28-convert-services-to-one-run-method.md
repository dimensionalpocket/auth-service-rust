# Convert Services to One-Struct-Per-Operation (`run`) Pattern

## Goal

Convert the codebase from “one Service struct with many methods” (e.g. `UserService::create_user`, `UserService::update_user`, etc.) to “one Service struct per operation with a single `run(...)` method” (e.g. `CreateUserService::run(...)`).

This should mirror the prior orchestrator migration: one file per operation, one struct per operation, one public entrypoint (`run`).

Constraints:

- Business logic must not change.
- Each new `run(...)` method must keep the same argument and return types as the old method it replaces.
- All callers must be updated (including unit tests in `src/` and integration tests in `tests/`).

## Non-goals

- No behavior changes.
- No API/schema changes.
- No new crates.
- No refactors beyond the mechanical conversion required to preserve compilation.

## Current Service Inventory (as of 2026-01-28)

The following files exist under `src/services/`:

- `src/services/auth_service.rs`
- `src/services/cookie_service.rs`
- `src/services/password_service.rs`
- `src/services/role_service.rs`
- `src/services/server_service.rs`
- `src/services/session_service.rs`
- `src/services/shutdown_service.rs`
- `src/services/site_service.rs`
- `src/services/user_service.rs`
- `src/services/mod.rs`

### `AuthService` methods (`src/services/auth_service.rs`)

- `login(conn: &mut SqliteConnection, username: &str, password: &str, session_secret: &[u8]) -> Result<AuthResult, SessionError>`
- `register(conn: &mut SqliteConnection, username: &str, password: &str, password_confirmation: &str, session_secret: &[u8]) -> Result<RegisterResult, UserError>`
- `get_current_user(conn: &mut SqliteConnection, session_context: &SessionContext) -> Result<AuthMeResult, SessionError>`

### `CookieService` methods (`src/services/cookie_service.rs`)

- `generate_session_cookie(config: &DpsAuthApiConfig, session_token: &str) -> String`
- `generate_logout_cookie(config: &DpsAuthApiConfig) -> String`

### `PasswordService` methods (`src/services/password_service.rs`)

- `generate(password: &str) -> Result<String, PasswordError>`
- `verify(password: &str, hash: &str) -> Result<bool, PasswordError>`

(Also includes private helpers: `detect_algorithm`, `verify_argon2`, `verify_bcrypt`, and `HashAlgorithm`.)

### `RoleService` methods (`src/services/role_service.rs`)

- `get_all_roles(conn: &mut SqliteConnection) -> Result<Vec<Role>, RoleError>`
- `get_role_by_id(conn: &mut SqliteConnection, role_id: i64) -> Result<Option<Role>, RoleError>`
- `get_role_by_name(conn: &mut SqliteConnection, name: &str) -> Result<Option<Role>, RoleError>`
- `create_role(conn: &mut SqliteConnection, create_data: CreateRoleData) -> Result<Role, RoleError>`
- `delete_role(conn: &mut SqliteConnection, role_id: i64) -> Result<Role, RoleError>`
- `update_role(conn: &mut SqliteConnection, role_id: i64, update_data: UpdateRoleData) -> Result<Role, RoleError>`
- `set_default_role(conn: &mut SqliteConnection, role_id: i64) -> Result<Role, RoleError>`
- `check_user_permission(conn: &mut SqliteConnection, user: &User, permission: &str) -> Result<bool, RoleError>`

### `ServerService` methods (`src/services/server_service.rs`)

- `get_server_timestamp() -> u64`

### `SessionService` methods (`src/services/session_service.rs`)

- `create_session(conn: &mut SqliteConnection, username: &str, password: &str, secret: &[u8]) -> Result<String, SessionError>`
- `create_session_for_user(user: &User, secret: &[u8]) -> Result<String, SessionError>`

### `ShutdownService` methods (`src/services/shutdown_service.rs`)

- `wait_for_shutdown_signal() -> &'static str` (async)
- `log_shutdown_start(signal_name: &str) -> ()`

### `SiteService` methods (`src/services/site_service.rs`)

- `create_site(conn: &mut SqliteConnection, data: CreateSiteData) -> Result<Site, SiteError>`
- `get_all_sites(conn: &mut SqliteConnection) -> Result<Vec<Site>, SiteError>`
- `update_site(conn: &mut SqliteConnection, id: i64, data: UpdateSiteData) -> Result<Option<Site>, SiteError>`
- `delete_site(conn: &mut SqliteConnection, site_id: i64) -> Result<Site, SiteError>`

(Also includes private helper: `validate_slug(slug: &str) -> Result<(), SiteError>`.)

### `UserService` methods (`src/services/user_service.rs`)

- `create_user(conn: &mut SqliteConnection, username: &str, password: &str) -> Result<User, UserError>`
- `get_user_by_name(conn: &mut SqliteConnection, name: &str) -> Result<Option<User>, sqlx::Error>`
- `get_user_by_id(conn: &mut SqliteConnection, user_id: i64) -> Result<Option<User>, sqlx::Error>`
- `update_password(conn: &mut SqliteConnection, user_id: i64, current_password: &str, new_password: &str, new_password_confirmation: &str) -> Result<User, UserError>`
- `delete_user(conn: &mut SqliteConnection, user_id: i64) -> Result<bool, UserError>`
- `update_user(conn: &mut SqliteConnection, user_id: i64, update_data: UpdateUserData, password: Option<String>) -> Result<User, UserError>`

(Also includes private helpers: `validate_username(username: &str) -> Result<(), UserError>` and `validate_password(password: &str) -> Result<(), UserError>`.)

## Target Structure and Naming

### Module layout

Adopt an orchestrator-like layout for services:

- `src/services/mod.rs` becomes a thin index of per-domain service modules.
- New domain modules:
  - `src/services/auth/`
  - `src/services/cookie/`
  - `src/services/password/`
  - `src/services/role/`
  - `src/services/server/`
  - `src/services/session/`
  - `src/services/shutdown/`
  - `src/services/site/`
  - `src/services/user/`

Each domain folder contains:

- `mod.rs` exporting the operation services for that domain.
- One file per operation, named in snake_case (matching orchestrator convention).

### Service naming

- One struct per operation, suffixed with `Service`.
- Each struct has exactly one public method:
  - `pub async fn run(...) -> Result<...>` for async operations
  - `pub fn run(...) -> ...` for sync operations

The `run(...)` function’s argument and return types match the old method it replaces.

## Operation Mapping (Old -> New)

This mapping is the core of the migration. Every old service method becomes one new service.

### Auth

- `AuthService::login(...)` -> `AuthLoginService::run(...)` in `src/services/auth/auth_login.rs`
- `AuthService::register(...)` -> `AuthRegisterService::run(...)` in `src/services/auth/auth_register.rs`
- `AuthService::get_current_user(...)` -> `AuthGetCurrentUserService::run(...)` in `src/services/auth/auth_get_current_user.rs`

### Cookie

- `CookieService::generate_session_cookie(...)` -> `GenerateSessionCookieService::run(...)` in `src/services/cookie/generate_session_cookie.rs`
- `CookieService::generate_logout_cookie(...)` -> `GenerateLogoutCookieService::run(...)` in `src/services/cookie/generate_logout_cookie.rs`

### Password

- `PasswordService::generate(...)` -> `GeneratePasswordHashService::run(...)` in `src/services/password/generate_password_hash.rs`
- `PasswordService::verify(...)` -> `VerifyPasswordService::run(...)` in `src/services/password/verify_password.rs`

Password helper handling:

- Codebase scan result (2026-01-28): `detect_algorithm`, `verify_argon2`, `verify_bcrypt`, and `HashAlgorithm` are only used inside `src/services/password_service.rs` (including its in-file unit tests).
- Decision: do NOT extract these helpers into standalone services.
  - Keep `HashAlgorithm`, `detect_algorithm`, `verify_argon2`, and `verify_bcrypt` as private implementation details inside `VerifyPasswordService` (`src/services/password/verify_password.rs`).
  - Move/port the existing `password_service.rs` tests that exercise `detect_algorithm` into `src/services/password/verify_password.rs` so they still have access to the private helpers.

### Role

- `RoleService::get_all_roles(...)` -> `GetAllRolesService::run(...)` in `src/services/role/get_all_roles.rs`
- `RoleService::get_role_by_id(...)` -> `GetRoleByIdService::run(...)` in `src/services/role/get_role_by_id.rs`
- `RoleService::get_role_by_name(...)` -> `GetRoleByNameService::run(...)` in `src/services/role/get_role_by_name.rs`
- `RoleService::create_role(...)` -> `CreateRoleService::run(...)` in `src/services/role/create_role.rs`
- `RoleService::delete_role(...)` -> `DeleteRoleService::run(...)` in `src/services/role/delete_role.rs`
- `RoleService::update_role(...)` -> `UpdateRoleService::run(...)` in `src/services/role/update_role.rs`
- `RoleService::set_default_role(...)` -> `SetDefaultRoleService::run(...)` in `src/services/role/set_default_role.rs`
- `RoleService::check_user_permission(...)` -> `CheckUserPermissionService::run(...)` in `src/services/role/check_user_permission.rs`

### Server

- `ServerService::get_server_timestamp()` -> `GetServerTimestampService::run()` in `src/services/server/get_server_timestamp.rs`

### Session

- `SessionService::create_session(...)` -> `CreateSessionService::run(...)` in `src/services/session/create_session.rs`
- `SessionService::create_session_for_user(...)` -> `CreateSessionForUserService::run(...)` in `src/services/session/create_session_for_user.rs`

### Shutdown

- `ShutdownService::wait_for_shutdown_signal()` -> `WaitForShutdownSignalService::run()` in `src/services/shutdown/wait_for_shutdown_signal.rs`
- `ShutdownService::log_shutdown_start(...)` -> `LogShutdownStartService::run(...)` in `src/services/shutdown/log_shutdown_start.rs`

### Site

- `SiteService::validate_slug(slug: &str) -> Result<(), SiteError>` -> `ValidateSiteSlugService::run(slug: &str) -> Result<(), SiteError>` in `src/services/site/validate_site_slug.rs` (new public operation + its own tests)
- `SiteService::create_site(...)` -> `CreateSiteService::run(...)` in `src/services/site/create_site.rs`
- `SiteService::get_all_sites(...)` -> `GetAllSitesService::run(...)` in `src/services/site/get_all_sites.rs`
- `SiteService::update_site(...)` -> `UpdateSiteService::run(...)` in `src/services/site/update_site.rs`
- `SiteService::delete_site(...)` -> `DeleteSiteService::run(...)` in `src/services/site/delete_site.rs`

### User

- `UserService::validate_username(username: &str) -> Result<(), UserError>` -> `ValidateUserNameService::run(username: &str) -> Result<(), UserError>` in `src/services/user/validate_user_name.rs`
- `UserService::validate_password(password: &str) -> Result<(), UserError>` -> `ValidateUserPasswordService::run(password: &str) -> Result<(), UserError>` in `src/services/user/validate_user_password.rs`
- `UserService::create_user(...)` -> `CreateUserService::run(...)` in `src/services/user/create_user.rs`
- `UserService::get_user_by_name(...)` -> `GetUserByNameService::run(...)` in `src/services/user/get_user_by_name.rs`
- `UserService::get_user_by_id(...)` -> `GetUserByIdService::run(...)` in `src/services/user/get_user_by_id.rs`
- `UserService::update_password(...)` -> `UpdateUserPasswordService::run(...)` in `src/services/user/update_user_password.rs`
- `UserService::delete_user(...)` -> `DeleteUserService::run(...)` in `src/services/user/delete_user.rs`
- `UserService::update_user(...)` -> `UpdateUserService::run(...)` in `src/services/user/update_user.rs`

## Implementation Plan

### Phase 1: Introduce new service modules (no call site changes yet)

1. Create new domain folders and `mod.rs` files under `src/services/`.
2. For each old service method listed in the mapping:
   - Create a new file and struct.
   - Move the method body into `run(...)`.
   - Keep helper logic local to that service where needed.

Notes on helpers and “implementation stays the same”:

- For site and user validation helpers:
  - Promote them to first-class operations (see mapping): `ValidateSiteSlugService`, `ValidateUserNameService`, `ValidateUserPasswordService`.
  - Update all relevant services to call these validations via `...Service::run(...)` rather than keeping local private copies.
- For password helper functions:
  - Apply the “multiple callers” rule described in the Password section above.

### Phase 2: Update `src/services/mod.rs` exports

1. Replace the current flat module exports in `src/services/mod.rs` with domain modules.
2. Re-export the new operation services so existing modules can import from `crate::services::...`.
3. Keep existing type exports that are not being renamed (e.g. `AuthResult`, `RegisterResult`, `AuthMeResult`) in the auth domain module and re-export them from `src/services/mod.rs`.
4. Keep the `SessionPayload` re-export unchanged.

### Phase 3: Update all call sites

Mechanically update every call site to use the new `*Service::run(...)` APIs.

Primary affected areas:

- GraphQL resolvers: `src/graphql/resolvers/**/*.rs`
- Orchestrators: `src/orchestrators/**/*.rs`
- Test utilities: `src/test_utils/mod.rs`
- Unit tests embedded in `src/**` modules
- Integration tests in `tests/**/*.rs`

Expected replacement examples:

- `UserService::create_user(...)` -> `CreateUserService::run(...)`
- `RoleService::check_user_permission(...)` -> `CheckUserPermissionService::run(...)`
- `SessionService::create_session(...)` -> `CreateSessionService::run(...)`
- `CookieService::generate_logout_cookie(...)` -> `GenerateLogoutCookieService::run(...)`

### Phase 4: Remove old flat service files

After all call sites are updated and compilation passes:

1. Remove the old service modules:
   - `src/services/auth_service.rs`
   - `src/services/cookie_service.rs`
   - `src/services/password_service.rs`
   - `src/services/role_service.rs`
   - `src/services/server_service.rs`
   - `src/services/session_service.rs`
   - `src/services/shutdown_service.rs`
   - `src/services/site_service.rs`
   - `src/services/user_service.rs`
2. Ensure `src/services/mod.rs` no longer references them.

### Phase 5: Fix compilation edges and imports

1. Normalize imports so call sites prefer specific imports over `use crate::services::*;`.
2. Ensure no “full paths in function bodies” regressions are introduced.
3. Resolve any cyclic imports by moving shared types (e.g. `AuthResult` structs) into a stable module location (`src/services/auth/types.rs` if needed).

### Phase 6: Verify with tests + lint

1. Run tests:

```bash
cargo test --quiet
```

2. Run lint + fmt (as configured):

```bash
cargo clippy --allow-dirty --fix && cargo fmt
```

3. Re-run tests:

```bash
cargo test --quiet
```

## Acceptance Criteria

- No remaining references to the old multi-method service structs (e.g. `UserService::`, `RoleService::`, etc.).
- All tests (unit + integration) pass.
- `cargo clippy --allow-dirty --fix && cargo fmt` succeeds with no changes needed afterward.
- Behavior remains unchanged (only structural refactor).
