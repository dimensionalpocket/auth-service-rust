# AGENTS.md

## Build/Lint/Test Commands

- **Build**: `cargo build`
- **Build release**: `cargo build --release`
- **Test all**: `cargo test --quiet`
- **Test single test**: `cargo test <test_name> --verbose`
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
- If plan involves creating or updating GraphQL resolvers, add a TODO to update the `README.md` mutations/queries table
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


## Project Structure

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
- Maintaining the schema:
  - The schema lives in `src/graphql/schema.rs`
  - Add new queries to the `Query` struct and new mutations to the `Mutation` struct
  - Do not change the implementations of `Query::new()` and `Mutation::new()`
  - The `QueryRoot` delegation pattern is not used in this project

### Orchestration Layer
- Service orchestrators live in `src/orchestrators/`
- Orchestrators call multiple services to implement complete business operations
- They handle the common pattern of: authentication → authorization → business logic
- Orchestrators are used when a resolver needs to call two or more services, or when the resolver requires authentication/authorization checks

### Service Layer
- Services live in `src/services/`
- Services contain core business logic and interact with the database layer via Query objects
  - Services should not contain SQL queries directly; if a query doesn't exist, create a new Query object in `src/queries/`
- Services can also call other services as needed

### Database
- Query objects live in `src/queries/`
  - Use `sqlx` to build dynamic queries
- The objects returned by queries (models) live in `src/models/`
- The Database instance is managed by `Database` struct in `src/database/mod.rs`
- Use async/await for database operations
- Database migrations live in `config/database/migrations`
  - Each migration has two files: `.sql` (forward) and `.down.sql` (rollback)
  - Migration files contain native SQL code

### Role Permissions

- All valid role permissions are defined in a static array at `src/models/user_role.rs:ROLE_PERMISSIONS`. This array serves as the whitelist of all allowed permissions in the system.
- When adding new permissions or removing existing ones:
  1. Update the `ROLE_PERMISSIONS` array in `src/models/user_role.rs`
  2. The `is_valid_role_permission()` function will automatically validate against the updated array
  3. All permission checks throughout the codebase use this validation
- Permission checks are performed through `UserRoleService::check_user_permission()` which validates that the permission exists in the static array before checking the user's role.

### Testing

- Unit tests live in the same files as the code they test, within `#[cfg(test)]` modules
- Integration and other higher-level tests live in the `tests/` directory

## Code Style Guidelines

### General
- Bias towards simplicity
- Do not install any new crates unless they are specified in the plan

### Formatting
- Follow the rules configured in `rustfmt.toml`
- Follow rustfmt defaults for all other formatting

### Imports
- Prefer specific imports over `use *;`

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

### Testing
- Test both success and error paths
- Use `tokio::test` for async tests
- Use `serial_test` crate for tests that rely on mutable ENV variables, or tests that otherwise cannot run in parallel

Rules for database usage in tests:

- Use tempfile for test databases
- **Pool Usage**: Exactly one SQLite pool per test - never multiple pools within single tests
- **Unit Tests**: Use direct pool access via `create_test_database()` from `src/database/mod.rs`
- **Configurable Tests**: Use `create_test_database_with_config(configure_sqlite: bool)` for optional SQLite configuration
- **Integration Tests**: Use full app with embedded pool via `create_app()` in test files
- **Ownership**: Clean ownership with automatic temp file cleanup via `NamedTempFile` dropping
- **Pool Size**: Unit tests use defaults, integration tests set to 1 connection
- **No Multiple Pools**: No tests use multiple pools within the same test function
- **Arc Usage**: Not needed in tests - `SqlitePool` implements `Clone` internally and tests use single-threaded `&pool` references
