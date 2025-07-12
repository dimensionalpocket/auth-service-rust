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

- [x] Implement `SessionService` for managing session tokens
  - [x] We're using a custom encrypted token format, not JWT
    - Why not JWT? Because we don't want to expose the payload structure to consumers
    - Consumers will call a future `getCurrentSession` query to retrieve the session payload with the token in the request (header or cookie)
  - [x] Define the payload structure for the session token - JSON-serializable
    - Fields: `sub` (subject - user id), `iat` (issued at timestamp), `exp` (expiration timestamp)
  - [x] Requires a secret key for signing tokens, stored in an environment variable (`DP_AUTH_SECRET_KEY`)
    - Determine how the service will act if the secret key is not set
  - [x] `encode_token` method to create a session token from a payload
  - [x] `decode_token` method to decode a session token and retrieve the payload
- [x] Test the `SessionService` methods
- [x] Document the `SessionService` methods via Rust doc comments

### Phase 9: Session Middleware

- [x] Implement middleware for session token management, to be used in the GraphQL API only (all queries and mutations)
- [x] Middleware should:
  - [x] Check for the session token in the request header or cookie, in that order (if both present, prefer the header)
    - Do not fallback to cookie if the header token is present but invalid
  - [x] Decode the token using `SessionService::decode_token`
  - [x] If valid, attach the decoded session token payload to the request context
  - [x] If invalid or missing, don't attach the payload and allow the request to proceed without it (each resolver will handle the absence of the payload)
- [x] Decisions to make:
  - [x] What is the prefix for the auth header, considering it's a custom encrypted token? ("Bearer" or something else?)
  - [x] Do we need to know the cookie name? If so, use `DpAuthSession` as the cookie name and store this string in a constant
- [x] Allow resolvers to have access to the session token payload via the request context
  - Resolvers can then propagate the session token payload to the Service objects on a case-by-case basis
- [x] Unit tests to ensure the middleware is attaching the session token payload when valid
  - Integration tests will be implemented in a later phase, by resolvers that actually use the session token payload

### Phase 10: `SessionService::create_session` Method

- [x] Accepts username and password as input
- [x] Calls `UserService::get_user_by_name` to retrieve the user by username
- [x] Calls `PasswordService::verify` to check the password against the stored hash
- [x] If valid, calls `SessionService::encode_token` to create a session token
- [x] Returns the session token, or an error if the credentials are invalid
  - [x] Error message should be specific (e.g., "User is blank" or "User not found" or "Password is blank" or "Password does not match", etc)
  - [x] Errors should be logged using the existing logging system and must contain the given username (not the password) for debugging
  - [x] Errors should be logged at `info` level, not `warn` or `error` -- those errors should not raise alarms in our logs, they're just infomational
- [x] Verify existing CORS configuration for 'with_credentials' support
- [x] Unit tests for the `create_session` method
- [x] Document the `create_session` method via Rust doc comments

### Phase 11: `createSession` Mutation (sign-in)

- [ ] Implement `createSession` GraphQL mutation
  - [ ] Accepts username and password as input
  - [ ] Calls `SessionService::create_session`
    - [ ] On success, returns the session token and sets the cookie with the token in the response
    - [ ] On failure, returns a user-friendly error message (not the internal error message), e.g., "Invalid credentials" for any username or password error, or "Internal server error" for unexpected errors
  - [ ] Make a decision if errors should return 2XX or 4XX status codes, as it impacts the client-side error handling
- [ ] Unit tests for the `createSession` mutation, ensuring it calls the `SessionService::create_session` method with the correct parameters
- [ ] Integration tests for the `createSession` mutation, ensuring it returns a session token and sets the cookie in the response

### Phase 12: `getCurrentSession` Query

- [ ] Returns the current session token payload, set by the session middleware
- [ ] Make a decision if the query should return 2XX or 401 status code if the session token is not present or invalid
- [ ] Unit tests for the `getCurrentSession` query
- [ ] Integration tests for the `getCurrentSession` query

## Future Phases

- Email support
- Cookie-less session management (using custom headers in response)
- Password change (when logged in)
- Password reset (requires email support)
- Username change
- User deletion/redaction
- `user_sessions` table to track user sessions and make tokens revocable
