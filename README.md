# Dimensional Pocket - Auth Service

This document outlines the project configuration and roadmap for the Rust-based GraphQL Auth Service for Dimensional Pocket. The service will provide REST endpoints, a GraphQL API, and user management functionalities.

## Project Configuration

- **Project Name**: `dp-auth-service`
- **Indentation**: Use 2 spaces for Rust code (not the standard 4 spaces)
- **GraphQL Library**: `async-graphql`
- **Database**: `sqlx` with SQLite on a file in the repo root's `data` directory (will be mounted in production)
- **REST endpoints**: `/` and `/health`, via `axum`
- **GraphQL Endpoint**: `/graphql`
- **Async Runtime**: `tokio`
- **Password Hashing**: `argon2`; 16-byte salt generated with `rand::rngs::OsRng`
- **Design Patterns**:
  - GraphQL queries and mutations only call Service objects and handle the response
    - E.g., `getServerTimestamp` query calls `ServerService::get_server_timestamp`
    - Service objects are static and do not require instantiation
    - Service objects have their own unit tests with full coverage
    - GraphQL Query and Mutation tests only test if they're calling the Service methods with the correct parameters
    - Service objects are stored in `src/services`
  - SQL queries are executed via Query objects
    - E.g., `UserByIdQuery::run(user_id)`
    - All query objects have a `run` method (arguments may vary) that executes the query and returns the result
    - Query objects are static and do not require instantiation
    - Query objects are stored in `src/queries`

## Roadmap

### Phase 1: Project Setup w/ REST Endpoints + `getServerTimestamp` query

- [ ] Initialize Rust project
- [ ] Configure `async-graphql`, `axum`, `sqlx`, and `tokio`
- [ ] Create REST endpoints:
  - [ ] `/` - Returns a simple "OK" message
  - [ ] `/health` - Returns a simple "OK" message
- [ ] Implement `getServerTimestamp` GraphQL query and associated Service object:
  - [ ] Returns the current server timestamp (milliseconds since epoch)
- [ ] Test the REST endpoints, `getServerTimestamp` query, and the Service object
- [ ] Document the API endpoints and query

### Phase 2: Password Service

- [ ] Implement `PasswordService` for user password management
  - [ ] `generate` method to create a new password hash
    - Already includes the salt generation
  - [ ] `verify` method to check a password against a hash
- [ ] Test the `PasswordService` methods
- [ ] Document the `PasswordService` methods via Rust doc comments

### Phase 3: Database Configuration

- [ ] Configure `sqlx` to use SQLite database in `data/development.db` (filename to come from environment variable)
  - [ ] Potentially use `dotenv`
- [ ] Create database schema with `sqlx` migrations
  - [ ] Create migration for `users` table
  - [ ] Schema to be defined as part of this task
  - [ ] Migrations to be stored in `config/database/migrations`
- [ ] Run migrations to create the database schema
- [ ] Verify support for schema dump after running migrations, to be stored in `config/database/schema.sql`

