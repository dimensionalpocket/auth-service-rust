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


## Code Style Guidelines

### General
- Bias towards simplicity
- Do not install any new crates unless they are specified in the plan

### Formatting
- Follow the rules configured in `rustfmt.toml`
- Follow rustfmt defaults for all other formatting

### Imports
- Group imports logically: std, external crates, internal modules
- Use `crate::` for internal module references
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
- Always add password parameters to the `skip` list in `#[instrument]` macros
- Use `#[instrument(skip(self, ctx, password, password_confirmation), fields(username = %username))]` pattern for auth functions
- Only log non-sensitive identifiers like usernames for debugging purposes
- Verify no sensitive data is logged by running the password logging tests

### Testing
- Write unit tests in #[cfg(test)] modules
- Use tempfile for test databases
- Test both success and error paths
- Use `tokio::test` for async tests
- Use `serial_test` crate for tests that rely on mutable ENV variables, or tests that otherwise cannot run in parallel
