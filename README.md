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
    - Each GraphQL query and mutation are implemented in independent files in `src/graphql/queries` and `src/graphql/mutations` respectively
      - Files are named directly after the query or mutation they implement, e.g., `get_server_timestamp.rs` for the `getServerTimestamp` query
      - The structs for the queries and mutations are suffixed with `Query` or `Mutation`
        - E.g., `GetServerTimestampQuery` for the `getServerTimestamp` query
  - SQL Queries are executed via SQL Query objects
    - E.g., `UserByIdQuery::run(user_id)`
    - All SQL query objects have a `run` method (arguments may vary) that executes the database query and returns the result
    - SQL Query objects are static and do not require instantiation
    - SQL Query objects are stored in `src/queries`

## Roadmap

### Phase 1: Project Setup w/ REST Endpoints + `getServerTimestamp` query

- [x] Initialize Rust project
- [x] Configure `async-graphql`, `axum`, `sqlx`, and `tokio`
- [x] Create REST endpoints:
  - [x] `/` - Returns a simple "OK" message
  - [x] `/health` - Returns a simple "OK" message
- [x] Implement `getServerTimestamp` GraphQL query and associated Service object:
  - [x] Returns the current server timestamp (milliseconds since epoch)
- [x] Test the REST endpoints, `getServerTimestamp` query, and the Service object
- [x] Document the API endpoints and query

### Phase 2: Logging

- [x] Suggest industry standards for logging endpoints and queries in Rust/GraphQL
- [x] Implement logging for existing REST endpoints and GraphQL queries (check Phase 1 for a list of endpoints and queries)
  - [x] For REST endpoints, log the request method, path, response status, and response time
  - [x] For GraphQL queries, log the operation name and response time (no parameters)

### Phase 3: Password Service

- [x] Implement `PasswordService` for user password management
  - [x] `generate` method to create a new password hash
    - Already includes the salt generation
  - [x] `verify` method to check a password against a hash
- [x] Test the `PasswordService` methods
- [x] Document the `PasswordService` methods via Rust doc comments

### Phase 4: Database Configuration

- [x] Configure `sqlx` to use SQLite database in `data/development.db` (filename to come from environment variable)
- [x] Create database schema with `sqlx` migrations
  - [x] Create migration for `user_roles` table
  - [x] Create migration for `users` table
  - [x] Schema defined with proper foreign key constraints
  - [x] Migrations stored in `config/database/migrations`
- [x] Run migrations to create the database schema
- [x] Verify support for schema dump after running migrations, to be stored in `config/database/schema.sql`
- [x] Implement SQL Query objects following project patterns
- [x] Create comprehensive unit tests for all query objects
- [x] Create integration tests demonstrating complete user creation flow
- [x] Implement seeds system with default user roles

### Phase 5: User Roles and Permissions

- [x] Suggest industry standards for user roles and permissions in Rust/GraphQL
- [x] Permissions are granular (e.g., `can_create_user`, `can_delete_user`, etc.) and can be assigned to roles
- [x] Update the database schema to store permissions for roles (details to be defined)
- [x] Implement `UserRoleService` for managing user roles and checking permissions
  - [x] `get_role_by_id` method to retrieve a role by ID
  - [x] `get_role_by_name` method to retrieve a role by name
  - [x] `check_permission` method to check if a user has a specific permission

### Phase 6: User Service - User Creation and Retrieval

- [x] Implement `UserService` for user management
  - [x] `create_user` method to create a new user
    - [x] Validates input (username and password) and checks for username already in use
    - [x] Uses `PasswordService` to hash the password
    - [x] Assigns default role to the user
  - [x] `get_user_by_id` method to retrieve a user by ID
  - [x] `get_user_by_name` method to retrieve a user by name

### Phase 7: `createUser` Mutation

- [x] Implement `createUser` GraphQL mutation
  - [x] Calls `UserService::create_user`
  - [x] Returns the created user object
- [x] Unit tests for the `createUser` mutation
  - [x] Tests should verify that the mutation calls the `UserService::create_user` method with the correct parameters
- [x] Integration tests for the `createUser` mutation
  - [x] Call the endpoint to create a user

### Phase 8: Session Service - Token Management

- [ ] Implement `SessionService` for managing session tokens
  - [ ] We're using a custom encrypted token format, not JWT
    - Why not JWT? Because we don't want to expose the payload structure to consumers
    - Consumers will call a future `getCurrentSession` query to retrieve the session payload with the token in the request (header or cookie)
  - [ ] Define the payload structure for the session token - JSON-serializable
    - Fields: `sub` (subject - user id), `iat` (issued at timestamp), `exp` (expiration timestamp)
  - [ ] Requires a secret key for signing tokens, stored in an environment variable (`DP_AUTH_SECRET_KEY`)
    - Determine how the service will act if the secret key is not set
  - [ ] `encode_token` method to create a session token from a payload
  - [ ] `decode_token` method to decode a session token and retrieve the payload
- [ ] Test the `SessionService` methods
- [ ] Document the `SessionService` methods via Rust doc comments

### Phase 9: Session Middleware

- [ ] Implement middleware for session token management, to be used in the GraphQL API only (all queries and mutations)
- [ ] Middleware should:
  - [ ] Check for the session token in the request header or cookie
  - [ ] Decode the token using `SessionService::decode_token`
  - [ ] If valid, attach the session token payload to the request context
  - [ ] If invalid or missing, don't attach the payload and allow the request to proceed without it (each resolver will handle the absence of the payload)
- [ ] Allow resolvers to have access to the session token payload via the request context
  - Resolvers can then propagate the session token payload to the Service objects on a case-by-case basis
- [ ] Unit tests to ensure the middleware is attaching the session token payload when valid
  - Integration tests will be implemented in a later phase, by resolvers that actually use the session token payload

