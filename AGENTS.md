# AGENTS.md

## General Instructions

- Be concise and direct.
- Treat all questions as genuine inquiries for information; don't interpret questions as implying mistakes.

## Build/Lint/Test Commands

- **Build**: `cargo build`
- **Build release**: `cargo build --release`
- **Test all**: `cargo test --quiet`
- **Test single test**: `RUST_BACKTRACE=1 cargo test <test_name> --quiet`
  - Note: never use `--verbose` as it clutters output
- **Linter**: `cargo clippy --allow-dirty --fix && cargo fmt`
- **Run local server**: `cargo run --bin run_local_server`
- **Database migrations**: `cargo run --bin dps-auth-api-migrate`

## Working Guidelines

### Before Starting Tasks
- Read the `README.md` to get project context and progress
- When needing current date/time, run `date +%Y-%m-%d@%H:%M` for YYYY-MM-DD@HH:MM format (24-hour, no AM/PM)
- Git operations (commits, branches, merges, rebases) are NOT part of agent tasks

### Working with Plans
- Save plans in `docs/llm/plans/YYYY/MM/` directory structure
- Name files as `DD-task-name.md` (day + dash + lowercase task with dashes)
- Include implementation details: files to modify/create, code samples for new functions
- If applicable, suggest existing crates that can help with the implementation during plan creation; if multiple crates exist, list pros/cons and recommend one
- Never include git operations, changelogs, PR descriptions, version bumps, or release notes in plans
- Never worry about backwards compatibility if the version is pre-1.0.0
- If the plan involves creating, updating, or deleting GraphQL resolvers, add a final phase on the plan to update the `README.md` mutations/queries table after all implementation is done
- When writing plans that involve creating resolvers, queries, services, and/or orchestrators in the same plan, split into sub-tasks in this order, where each subtask is fully tested before moving to the next:
  - Queries first (if any)
  - Services second (if any)
  - Orchestrators third (if any)
  - GraphQL resolvers last
- Do not code immediately after writing plans - wait for user review/approval

### Implementing Plans
- Only start implementation after explicit user command
- During implementation, STOP and inform user if deviating from the plan in any way
- Never implement anything not explicitly mentioned in the plan
- After implementation is fully finished and all tests pass, run the linter command and fix any issues

## Tool Usage

### File Editing
When using the Edit tool to modify files:
- Preserve ALL existing comments and documentation
- Make surgical, targeted edits - only change the specific lines that need to change
- Comments that are unrelated to your change must remain intact

## Project Structure

### Domains

- `auth` - workflows related to authentication such as sign-up, login/logout, password recovery, etc
- `user` - user CRUD operations
- `role` - role CRUD operations and permission checks (authorization)
- `site` - site CRUD operations

### API Setup
- API handlers live in `src/handlers/`
- API has REST and GraphQL endpoints
  - REST is only used for health checks, all other functionality is via GraphQL

### Middleware
- Middlewares live in `src/middleware/`
- Middlewares handle session object injection, request ID injection, and logging
  - Session object and request ID are available to all GraphQL resolvers via context
- Middlewares never halt the request; they only add context data

### GraphQL
- GraphQL resolvers (both queries and mutations) live in `src/graphql/resolvers/`
- All resolvers are suffixed with `Resolver` (e.g., `AuthLoginResolver`), regardless of being query or mutation
- All resolver filenames are in snake_case matching the resolver name without the suffix (e.g., `auth_login.rs`)
- Resolvers should be thin wrappers that call **at most** a single service or orchestrator method
- Resolvers handle GraphQL-specific concerns (input validation, response formatting)
  - They do NOT contain business logic, database access, or session management
- Any objects that are part of the GraphQL context should be retrieved from the context object passed into the resolver method then passed to the orchestration/service layer as needed
- **GraphQL Types**: All GraphQL response types should be defined inline within their respective resolver files, not in separate types modules
  - This follows the established pattern where types live alongside the code that uses them
  - Do NOT create separate `types` modules for GraphQL types
  - Examples: `UserListing` in `users.rs`, `SiteListing` in `sites.rs`, `AddSiteResponse` in `add_site.rs`
- Naming conventions:
  - **Query/Mutation names**: camelCase with `#[graphql(name = "camelCase")]` (e.g., `updateUser`, `deleteUser`, `authLogin`)
  - **Field names**: snake_case in Rust with `#[graphql(name = "camelCase")]` for GraphQL output (e.g., `role_id` → `roleId`, `created_ts` → `createdTs`)
  - **Input types**: camelCase field names with `#[graphql(name = "camelCase")]` annotations
  - **Response types**: Follow same pattern as existing `AddSiteResponse`, `UpdateSiteResponse`, `AuthLoginResponse`
  - Note: async-graphql automatically converts snake_case field names to camelCase in the GraphQL schema, but explicit `#[graphql(name = "...")]` annotations are preferred for clarity and consistency
- Maintaining the schema:
  - The schema lives in `src/graphql/schema.rs`
  - Add new queries to the `Query` struct and new mutations to the `Mutation` struct
  - Do not change the implementations of `Query::new()` and `Mutation::new()`
  - The `QueryRoot` delegation pattern is not used in this project

### Orchestration Layer
- Service orchestrators live in `src/orchestrators/`
  - One orchestrator per domain, with multiple methods each
- Orchestrators are called by GraphQL resolvers only
- Orchestrators handle the common pattern of: authentication → authorization → business logic (calling other services)
- Orchestrator inputs are typically the database pool, session context (extracted by the resolver), and any resolver inputs
- Orchestrators extract a connection from the pool and use that connection to call any services and queries it needs

### Service Layer
- Services live in `src/services/`
  - One service per domain (e.g., Auth) or specialization (e.g., Password) with multiple methods
- Services contain core business logic and can interact with the database layer via Query objects
  - Services should not contain SQL queries directly; if a query doesn't exist, create a new Query object in `src/queries/`
- Services can also call other services as needed
- Service inputs are a database connection (not a pool) and any parameters needed for the business logic

### Database
- Query objects live in `src/queries/`
  - Use `sqlx` to build dynamic queries
- Query objects work with a database connection (not a pool)
- The objects returned by queries (models) live in `src/models/`
- The Database instance is managed by `Database` struct in `src/database/mod.rs`
- Use async/await for database operations
- Database migrations live in `config/database/migrations`
  - Each migration has two files: `.sql` (forward) and `.down.sql` (rollback)
  - Migration files contain native SQL code

### Role Permissions

- All valid role permissions are defined in a static array at `src/models/role.rs:ROLE_PERMISSIONS`. This array serves as the whitelist of all allowed permissions in the system.
- When adding new permissions or removing existing ones:
  1. Update the `ROLE_PERMISSIONS` array in `src/models/role.rs`
  2. The `is_valid_role_permission()` function will automatically validate against the updated array
  3. All permission checks throughout the codebase use this validation
- Permission checks are performed through `RoleService::check_user_permission()` which validates that the permission exists in the static array before checking the user's role.

### Testing

- Unit tests live in the same files as the code they test, within `#[cfg(test)]` modules
- Integration and other higher-level tests live in the `tests/` directory
- Test both success and error paths
- Use `tokio::test` for async tests
- Use `serial_test` crate for tests that rely on mutable ENV variables, or tests that otherwise cannot run in parallel
- **Timestamp Testing**: Database timestamp columns (`created_ts`, `updated_ts`) store seconds since Unix epoch. Any tests requiring timestamp differences must use delays of at least 1 second (e.g., `tokio::time::sleep(tokio::time::Duration::from_secs(1)).await`)

## Code Style Guidelines

### General
- Bias towards simplicity
- Do not install any new crates unless they are specified in the plan

### Formatting
- Follow the rules configured in `rustfmt.toml`
- Follow rustfmt defaults for all other formatting

### Imports
- Prefer specific imports over `use *;`
- **Never use full paths in function bodies** - always import first
- **Exceptions**: Types named "Error" should use full paths in code (e.g., `sqlx::Error`) and don't need to be imported directly

```rust
use crate::models::Role;
use crate::services::UserService;
use crate::services::UserError;
use sqlx::{SqlitePool, Row};
use async_graphql::{Context, Object, Result};
use std::collections::HashMap;

fn example() -> Result<(), UserError> {
    let role = Role { ... };                    // ✅ Use imported type directly
    let pool = SqlitePool::connect(...).await?; // ✅ Use imported type directly
    let map = HashMap::new();                   // ✅ Use imported type directly
    // let role = crate::models::Role { ... };   ❌ No full paths
    // let error = sqlx::Error::RowNotFound;     // ✅ Error types use full paths
}
```

### Types & Naming
- Use PascalCase for structs, enums, and types
- Use snake_case for functions, variables, and modules
- Use SCREAMING_SNAKE_CASE for constants

### Error Handling
- Create custom error enums for each service (e.g., UserError, SessionError)
- Implement Display and Error traits for custom errors
- Use From traits for error conversion
- Return Result<T, CustomError> from service methods
- Use ? operator for error propagation

### Logging Security
- **NEVER** log passwords, API keys, tokens, or other sensitive data
- Always add sensitive parameters to the `skip` list in `#[instrument]` macros
  - E.g.: `#[instrument(skip(self, ctx, password, password_confirmation), fields(username = %username))]` pattern for auth functions
- Only log non-sensitive identifiers like usernames for debugging purposes
- Verify no sensitive data is logged by running the password logging tests

### Test Utilities

- Shared test utilities are available in `src/test_utils/mod.rs`
- Use `create_test_user()`, `create_test_user_with_password()`, `create_test_user_full()`, `create_test_role()`, `create_test_role_model()`, and `create_test_user_via_mutation()` for creating test data
- Database setup utilities (`create_test_database()`, `create_test_database_with_pool_size()`, etc.) are also in `src/test_utils/mod.rs`
- Do not create local `create_test_*` functions in test modules - use the shared utilities instead

When writing resolver tests, use centralized `create_test_<query|mutation>_schema` helper from `test_utils` instead of direct `Schema::build` calls:

```rust
use crate::test_utils::{create_test_query_schema, create_test_mutation_schema};

// Query-only test (no context)
let schema = create_test_query_schema(query, None, None, None);

// Mutation with database and session
let schema = create_test_mutation_schema(mutation, Some(pool), Some(session), None);

// Mutation with database, session, and config
let schema = create_test_mutation_schema(mutation, Some(pool), Some(session), Some(config));
```

Rules for database usage in tests:

- Use tempfile for test databases
- **Pool Usage**: Exactly one SQLite pool per test - never multiple pools within single tests
- **Unit Tests**: Use direct pool access via `create_test_database()` from `src/test_utils/mod.rs`
- **Configurable Tests**: Use `create_test_database_with_config(configure_sqlite: bool)` for optional SQLite configuration
- **Integration Tests**: Use full app with embedded pool via `create_app()` in test files
- **Ownership**: Clean ownership with automatic temp file cleanup via `NamedTempFile` dropping
- **Pool Size**: Unit tests use defaults, integration tests set to 1 connection
- **No Multiple Pools**: No tests use multiple pools within same test function
- **Arc Usage**: Not needed in tests - `SqlitePool` implements `Clone` internally and tests use single-threaded `&pool` references
