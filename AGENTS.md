# AGENTS.md

## Build/Lint/Test Commands

- **Build**: `cargo build`
- **Build release**: `cargo build --release`
- **Test all**: `cargo test --verbose`
- **Test single test**: `cargo test <test_name>`
- **Format**: `cargo fmt` (uses 2-space indentation per rustfmt.toml)
- **Check formatting**: `cargo fmt -- --check`
- **Clippy lint**: `cargo clippy --allow-dirty --fix` (then run `cargo fmt`)
- **Run local server**: `cargo run --bin run_local_server`
- **Database migrations**: `cargo run --bin dps-auth-api-migrate`

## Working Guidelines

### Before Starting Tasks
- Read the README.md to get project context and progress
- When needing current date/time, run `date +%Y-%m-%d@%H:%M` for YYYY-MM-DD@HH:MM format (24-hour, no AM/PM)
- Git operations (commits, branches, merges, rebases) are NOT part of agent tasks

### Working with Plans
- Save plans in `docs/llm/plans/YYYY/MM/` directory structure
- Name files as `DD-task-name.md` (day + dash + lowercase task with dashes)
- Do not code immediately after writing plans - wait for user review/approval
- Include implementation details: files to modify/create, code samples for new functions
- During implementation, STOP and inform user if deviating from the plan
- Never implement anything not explicitly mentioned in the plan
- Never include git operations, changelogs, PR descriptions, version bumps, or release notes in plans
- Only worry about backwards compatibility for versions past 1.0.0



## Code Style Guidelines

### Formatting
- Use 2 spaces for indentation (configured in rustfmt.toml)
- Follow rustfmt defaults for all other formatting

### Imports
- Group imports logically: std, external crates, internal modules
- Use `crate::` for internal module references
- Prefer specific imports over `use *;`

### Types & Naming
- Use PascalCase for structs, enums, and types
- Use snake_case for functions, variables, and modules
- Use SCREAMING_SNAKE_CASE for constants
- Database models use i64 for IDs, String for UUIDs

### Error Handling
- Create custom error enums for each service (e.g., UserError, SessionError)
- Implement Display and Error traits for custom errors
- Use From traits for error conversion
- Return Result<T, CustomError> from service methods
- Use ? operator for error propagation

### Database
- Use sqlx with SQLite
- Implement FromRow for database models
- Use async/await for database operations
- Separate queries into dedicated modules
- Database migrations are in `config/database/migrations`
- Each migration has two files: `.sql` (forward) and `.down.sql` (rollback)
- Migration files contain native SQL code

### Testing
- Write unit tests in #[cfg(test)] modules
- Use tempfile for test databases
- Test both success and error paths
- Use tokio::test for async tests
