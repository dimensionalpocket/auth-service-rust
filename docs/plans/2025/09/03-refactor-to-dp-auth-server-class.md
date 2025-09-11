# Refactor to DpAuthServer Class

**Date**: 2025-09-03@11:44

## Overview

Completely refactor the current `start_server` function and `ServerConfig` struct approach to a more object-oriented design using a `DpAuthServer` class with a builder pattern. This is a breaking change that will provide better encapsulation, cleaner API, and more flexibility for server lifecycle management.

## Current State

The current implementation uses:
- `ServerConfig` struct with required parameters
- `start_server(config)` function that handles everything
- All server logic is in the `start_server` function in `src/lib.rs`
- Graceful shutdown is handled inline within the function

## Target State

The new implementation will use:
- `DpAuthServer` struct with builder pattern initialization
- `DpAuthServer::new()` method returning a builder
- Builder methods for all configuration options
- `server.start().await` method to start the server
- Internal methods to organize the current `start_server` logic
- Graceful shutdown handled within the server instance
- Complete removal of old `start_server` function and `ServerConfig`

## API Design

### New Usage Pattern

```rust
use dp_auth_service::DpAuthServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = DpAuthServer::new()
        .port(3000)
        .sqlite_file_path("data/production.db")
        .session_secret(your_32_byte_secret)
        .cookie_domain(".yourdomain.com")
        .insecure_cookie(false)
        .development_mode(false)
        .build()?;

    server.start().await
}
```

## Implementation Plan

### Phase 1: Create DpAuthServerBuilder Module (`src/dp_auth_server_builder.rs`)

Create the builder module first with all unit tests except the `build()` method:

```rust
// Note: DpAuthServer imports will be added in Phase 2
// For now, we'll create a placeholder error type for testing

#[derive(Debug, Default)]
pub struct DpAuthServerBuilder {
    port: Option<u16>,
    sqlite_file_path: Option<String>,
    session_secret: Option<Vec<u8>>,
    cookie_domain: Option<String>,
    insecure_cookie: Option<bool>,
    development_mode: Option<bool>,
}

impl DpAuthServerBuilder {
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn sqlite_file_path<S: Into<String>>(mut self, path: S) -> Self {
        self.sqlite_file_path = Some(path.into());
        self
    }

    pub fn session_secret(mut self, secret: Vec<u8>) -> Self {
        self.session_secret = Some(secret);
        self
    }

    pub fn cookie_domain<S: Into<String>>(mut self, domain: S) -> Self {
        self.cookie_domain = Some(domain.into());
        self
    }

    pub fn insecure_cookie(mut self, insecure: bool) -> Self {
        self.insecure_cookie = Some(insecure);
        self
    }

    pub fn development_mode(mut self, dev_mode: bool) -> Self {
        self.development_mode = Some(dev_mode);
        self
    }

    // build() method will be implemented in Phase 2 after DpAuthServer exists
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_port() {
        let builder = DpAuthServerBuilder::default().port(8080);
        assert_eq!(builder.port, Some(8080));
    }

    #[test]
    fn test_builder_sqlite_file_path() {
        let builder = DpAuthServerBuilder::default().sqlite_file_path("test.db");
        assert_eq!(builder.sqlite_file_path, Some("test.db".to_string()));
    }

    #[test]
    fn test_builder_session_secret() {
        let secret = vec![1u8; 32];
        let builder = DpAuthServerBuilder::default().session_secret(secret.clone());
        assert_eq!(builder.session_secret, Some(secret));
    }

    #[test]
    fn test_builder_cookie_domain() {
        let builder = DpAuthServerBuilder::default().cookie_domain(".example.com");
        assert_eq!(builder.cookie_domain, Some(".example.com".to_string()));
    }

    #[test]
    fn test_builder_insecure_cookie() {
        let builder = DpAuthServerBuilder::default().insecure_cookie(false);
        assert_eq!(builder.insecure_cookie, Some(false));
    }

    #[test]
    fn test_builder_development_mode() {
        let builder = DpAuthServerBuilder::default().development_mode(false);
        assert_eq!(builder.development_mode, Some(false));
    }

    #[test]
    fn test_builder_chaining() {
        let secret = vec![1u8; 32];
        let builder = DpAuthServerBuilder::default()
            .port(3000)
            .sqlite_file_path("data/test.db")
            .session_secret(secret.clone())
            .cookie_domain(".test.com")
            .insecure_cookie(true)
            .development_mode(true);

        assert_eq!(builder.port, Some(3000));
        assert_eq!(builder.sqlite_file_path, Some("data/test.db".to_string()));
        assert_eq!(builder.session_secret, Some(secret));
        assert_eq!(builder.cookie_domain, Some(".test.com".to_string()));
        assert_eq!(builder.insecure_cookie, Some(true));
        assert_eq!(builder.development_mode, Some(true));
    }

    #[test]
    fn test_builder_default() {
        let builder = DpAuthServerBuilder::default();
        assert_eq!(builder.port, None);
        assert_eq!(builder.sqlite_file_path, None);
        assert_eq!(builder.session_secret, None);
        assert_eq!(builder.cookie_domain, None);
        assert_eq!(builder.insecure_cookie, None);
        assert_eq!(builder.development_mode, None);
    }

    #[test]
    fn test_builder_string_conversion() {
        let builder = DpAuthServerBuilder::default()
            .sqlite_file_path(String::from("test.db"))
            .cookie_domain(String::from(".example.com"));

        assert_eq!(builder.sqlite_file_path, Some("test.db".to_string()));
        assert_eq!(builder.cookie_domain, Some(".example.com".to_string()));
    }
}
```

**Phase 1 Testing Requirements:**
- Run `cargo test` and ensure all builder unit tests pass
- Verify builder methods work correctly with method chaining
- Test that all field setters work as expected
- Confirm Default implementation works correctly

**✅ Phase 1 COMPLETED:**
- ✅ Created `src/dp_auth_server_builder.rs` with all builder methods
- ✅ Implemented 9 comprehensive unit tests for builder functionality
- ✅ All tests pass (9/9 builder tests, 119/119 total tests)
- ✅ Builder methods work correctly with method chaining
- ✅ Default implementation returns all None values as expected

### Phase 2: Create DpAuthServer Module (`src/dp_auth_server.rs`)

```rust
use crate::dp_auth_server_builder::DpAuthServerBuilder;

#[derive(Debug)]
pub struct DpAuthServer {
    config: ResolvedServerConfig,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedServerConfig {
    pub port: u16,
    pub sqlite_file_path: String,
    pub session_secret: Vec<u8>,
    pub cookie_domain: String,
    pub insecure_cookie: bool,
    pub development_mode: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DpAuthServerError {
    InvalidSecretLength { actual: usize, expected: usize },
    MissingRequiredConfig { field: String },
    DatabaseError(String),
    NetworkError(String),
    ConfigurationError(String),
}

impl DpAuthServer {
    pub fn new() -> DpAuthServerBuilder {
        DpAuthServerBuilder::default()
    }
}
```

**Phase 2 Tasks:**
- Update `src/dp_auth_server_builder.rs` to import and use `DpAuthServer` and `DpAuthServerError`
- Implement the `build()` method in `DpAuthServerBuilder`
- Add unit tests for the `build()` method (validation, error handling, defaults)
- Run `cargo test` and ensure all tests pass

**Phase 2 Completion Criteria:**
- ✅ Server can be built with only session_secret specified: `DpAuthServer::new().session_secret(secret).build()` should succeed
- ✅ All other fields should use correct defaults:
  - `port`: 3000
  - `sqlite_file_path`: "data/development.db"
  - `cookie_domain`: ".api.dp-auth.localhost"
  - `insecure_cookie`: false
  - `development_mode`: false
- ✅ When fields are explicitly defined, they should resolve correctly in the server config (override defaults)
- ✅ Missing session_secret should return appropriate error
- ✅ Invalid session_secret length should return appropriate error

**✅ Phase 2 COMPLETED:**
- ✅ Created `src/dp_auth_server.rs` with server struct and error types
- ✅ Implemented complete `build()` method with validation and defaults
- ✅ Added 5 comprehensive unit tests for build functionality
- ✅ Consolidated redundant tests for cleaner test suite
- ✅ All tests pass (14/14 builder tests, 124/124 total tests)
- ✅ All completion criteria met

### Phase 3: Implement Server Methods in DpAuthServer

**Phase 3 Overview:** This is the largest phase, implementing all server functionality. Break it down into sub-phases:

#### Phase 3A: Basic Server Infrastructure
- Implement `start()` method skeleton
- Implement `initialize_database()` method  
- Add unit tests for database initialization
- **Note**: Logging initialization removed - now caller's responsibility (global app concern)
- **Migrate applicable tests from `tests/integration_tests.rs`**: Database setup tests

**✅ Phase 3A COMPLETED:**
- ✅ Implemented `start()` method skeleton with database initialization
- ✅ Implemented `initialize_database()` method with proper error handling
- ✅ Removed logging initialization from server object (correct architecture)
- ✅ Added 5 comprehensive unit tests for database functionality
- ✅ All tests pass (19/19 builder/server tests, 129/129 total tests)
- ✅ Fixed tracing conflicts - logging is now caller's responsibility

#### Phase 3B: GraphQL Schema and Router Setup
- Implement `create_graphql_schema()` method
- Implement `build_router()` method with all routes and middleware
- Add unit tests for schema creation and router building
- **Migrate applicable tests from `tests/integration_tests.rs`**: Basic routing tests (`test_root_endpoint`, `test_health_endpoint`, `test_graphql_endpoint`, `test_404_handler_*`)

**✅ Phase 3B COMPLETED:**
- ✅ Implemented `build_router()` method with complete router configuration
- ✅ Router includes all routes: `/`, `/health`, `/graphql` (GET/POST), fallback (404)
- ✅ Router includes all middleware: Session, Request ID, Logging, CORS
- ✅ GraphQL configuration with proper handler setup using server config
- ✅ Removed unnecessary `create_graphql_schema()` wrapper method for cleaner code
- ✅ Added 7 comprehensive unit tests for router functionality
- ✅ Migrated all applicable routing tests from integration tests
- ✅ All tests pass (12/12 builder/server tests, 136/136 total tests)
- ✅ Updated `start()` method to use Phase 3B functionality

#### Phase 3C: Network and Shutdown Handling
- Implement `bind_listener()` method
- Implement `create_shutdown_handler()` method
- Complete the `start()` method implementation
- Add unit tests for network binding and shutdown handling

**✅ Phase 3C COMPLETED:**
- ✅ Implemented `bind_listener()` method with TCP listener creation and error conversion
- ✅ Implemented `create_shutdown_handler()` method using existing ShutdownService
- ✅ Completed `start()` method with full server lifecycle (database, router, network, shutdown)
- ✅ Added 2 focused unit tests for network binding functionality
- ✅ Removed problematic tests that would hang or test invalid scenarios
- ✅ All 14 dp_auth_server tests pass, 138 total tests pass
- ✅ Server now has complete functionality ready for end-to-end testing

#### Phase 3D: Integration Test Compatibility
- **✅ COMPLETED - Enhanced integration tests to use `DpAuthServer`**: 
  - ✅ Added public `create_app()` method to `DpAuthServer` for test usage
  - ✅ Made `build_router()` method public for advanced test scenarios
  - ✅ Updated `create_app()` test helper to use `DpAuthServer::create_app()`
  - ✅ Updated `create_app_with_database()` to use `DpAuthServer::build_router()` with custom schema
  - ✅ Removed manual router construction and test wrapper functions
  - ✅ Configured test servers with proper test settings (insecure cookies, development mode)
- **✅ All integration tests passing**: 
  - ✅ `test_create_session_mutation_success` - now uses real `DpAuthServer` router
  - ✅ `test_create_session_mutation_invalid_credentials` - now uses real `DpAuthServer` router
  - ✅ `test_create_session_mutation_with_missing_user` - now uses real `DpAuthServer` router
  - ✅ `test_get_current_session_integration_*` tests - now use real `DpAuthServer` router
- **✅ Improved test quality**: Tests now use production router configuration with all middleware layers

#### Phase 3E: Simplify Database Testing with Server-Managed Database
- **✅ Move schema dump functionality to Database struct**:
  - ✅ Move `generate_schema_dump_content()` function from `config/scripts/dp_auth_migrate.rs` to `src/database/mod.rs` as a public method `dump_schema_content()`
  - ✅ Move `generate_schema_dump()` function from `config/scripts/dp_auth_migrate.rs` to `src/database/mod.rs` as a public method `dump_schema_to_file(file_path: &str)`
  - ✅ The `dump_schema_to_file()` method takes a file path argument to allow dumping schema to any location
  - ✅ Add comprehensive unit tests for both schema dump methods in `src/database/mod.rs` (future-proof tests using custom test tables)
  - ✅ Update `config/scripts/dp_auth_migrate.rs` to use the new Database methods, passing `"config/database/schema.sql"` as the file path argument
  - ✅ Ensure schema dump functionality works exactly the same as before (writes to `config/database/schema.sql`) but with improved flexibility
- **✅ Separate database operations into distinct methods**:
  - ✅ Create new public `migrate_database()` method on `DpAuthServer` (migrations only)
  - ✅ Create new public `seed_database()` method on `DpAuthServer` (seeds only)
  - ✅ `initialize_database()` already only creates database connection, not migrations or seeds
  - ✅ Make `initialize_database()` method public for test usage
  - ✅ Add comprehensive unit tests for all three database methods (6 new tests)
  - ✅ Update `src/lib.rs` to export `DpAuthServer`, `DpAuthServerError`, and `DpAuthServerBuilder` to public API
- **✅ Simplify integration test database setup**:
  - ✅ Update `create_app_with_database()` to use `DpAuthServer` with temp file path (replaced with `create_app()` function)
  - ✅ Use `server.initialize_database()`, `server.migrate_database()`, and `server.seed_database()` in tests
  - ✅ Remove manual GraphQL schema creation with database injection
  - ✅ Use standard `server.create_app()` method instead of `build_router()`
  - ✅ Replace manual `setup_default_role()` and `UserService::create_user()` with seed data and test helper function
- **✅ Test pattern changes**:
  - ✅ Default roles come from `config/database/seeds/001_default_roles.sql` via `seed_database()`
  - ✅ Default users (including admin) come from `config/database/seeds/002_default_users.sql` via `seed_database()`
  - ✅ Remove manual `setup_default_role()` calls - use seeded roles instead (still present in some service tests, but integration tests are clean)
  - ✅ Regular test users created via `create_test_user_via_mutation()` test helper function (tests actual CreateUser mutation)
  - ✅ Admin functionality tests can use seeded admin user from `002_default_users.sql`
- **Benefits**:
  - Tests use exact same database initialization path as production
  - Clean separation: connection → migrations → seeds
  - More realistic testing using actual GraphQL mutations via test helper functions
  - Eliminates manual SQL and schema construction in tests
  - Uses existing seed infrastructure consistently

**Suggested Implementation Order:**
1. **Phase 3A** (Foundation) - Database and logging setup
2. **Phase 3B** (Core functionality) - Schema and routing 
3. **Phase 3C** (Network layer) - Binding and shutdown
4. **Phase 3D** (Integration) - Integration test compatibility
5. **Phase 3E** (Database testing) - Server-managed database for tests

**Phase 3 Completion Criteria:**
- ✅ Server can start successfully with `server.start().await`
- ✅ All individual server methods work correctly
- ✅ Database initialization works with configured path
- ✅ Router serves all endpoints correctly (/, /health, /graphql)
- ✅ GraphQL schema creation works
- ✅ Network binding works on configured port
- ✅ Graceful shutdown handling works
- ✅ All applicable integration tests migrated and passing
- ✅ Comprehensive unit test coverage for all server methods

### Phase 4: Final Migration and Cleanup

**Phase 4 Overview:** Remove old API, update binaries to use new API, ensure all tests pass

#### Phase 4A: Update Public API Exports
- Update `src/lib.rs` to export new API (`DpAuthServer`, `DpAuthServerBuilder`, `DpAuthServerError`)
- Remove old API exports (`start_server`, `ServerConfig`, `ConfigError`)

#### Phase 4B: Update run_local_server Binary
- Update `scripts/run_local_server.rs` to use new `DpAuthServer` API
- Add proper logging initialization to the binary
- Remove all references to old `start_server` function

#### Phase 4C: Remove Old Implementation
- Remove `start_server` function from `src/lib.rs`
- Remove `src/config.rs` file entirely (if no other content)
- Clean up any remaining old API references

#### Phase 4D: Final Testing and Validation
- Run full test suite and ensure all tests pass
- Test the updated `run_local_server` binary
- Verify no compilation errors with old API removed
- Confirm new API works end-to-end

**Phase 4 Completion Criteria:**
- ✅ Old API completely removed (`start_server`, `ServerConfig`, `ConfigError`)
- ✅ New API properly exported and working
- ✅ `run_local_server` binary uses new `DpAuthServer` API
- ✅ All tests pass (no regressions)
- ✅ No compilation errors
- ✅ End-to-end functionality verified

### Phase 5: Update README with New Examples

**Phase 5 Overview:** Update documentation to reflect new API and provide clear usage examples

#### Phase 5A: Update Usage Examples
- Replace all old `start_server` examples with new `DpAuthServer` API
- Provide three comprehensive examples:
  1. **Minimal Example**: Only session_secret specified (uses defaults)
  2. **Complete Example**: All configuration options specified
  3. **Logging Example**: How to add tracing initialization

#### Phase 5B: Update Installation and Configuration Sections
- Update code examples to use new API
- Remove references to old `ServerConfig` struct
- Update configuration documentation to reflect builder pattern
- Remove environment variables section (will be deprecated)

**Phase 5 Example Structure:**

**Example 1 - Minimal (defaults):**
```rust
use dp_auth_service::DpAuthServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = DpAuthServer::new()
        .session_secret(your_32_byte_secret)
        .build()?;

    server.start().await
}
```

**Example 2 - Complete configuration:**
```rust
use dp_auth_service::DpAuthServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = DpAuthServer::new()
        .port(3000)
        .sqlite_file_path("data/production.db")
        .session_secret(your_32_byte_secret)
        .cookie_domain(".yourdomain.com")
        .insecure_cookie(false)
        .development_mode(false)
        .build()?;

    server.start().await
}
```

**Example 3 - With logging:**
```rust
use dp_auth_service::DpAuthServer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dp_auth_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let server = DpAuthServer::new()
        .session_secret(your_32_byte_secret)
        .build()?;

    server.start().await
}
```

**Phase 5 Completion Criteria:**
- ✅ All old API examples replaced with new `DpAuthServer` API
- ✅ Three clear usage examples provided (minimal, complete, with logging)
- ✅ Configuration documentation updated for builder pattern
- ✅ Environment variables section clarified
- ✅ Installation section updated
- ✅ No references to old API remain

## Files to be Created/Modified

### New Files

1. **`src/dp_auth_server.rs`**
   - **ADD**: `DpAuthServer` struct
   - **ADD**: `ResolvedServerConfig` struct (internal)
   - **ADD**: `DpAuthServerError` enum
   - Implement all server methods (broken down from current `start_server` logic)
   - Unit tests for server functionality

2. **`src/dp_auth_server_builder.rs`**
   - **ADD**: `DpAuthServerBuilder` struct
   - Implement all builder pattern methods
   - Unit tests for builder functionality and validation

### Modified Files

1. **`src/lib.rs`**
   - **PHASE 1**: Add module declaration for `dp_auth_server_builder` only
   - **PHASE 2**: Add module declaration for `dp_auth_server`
   - **PHASE 6**: Remove `start_server` function completely and add re-exports for public API
   - Update exports to only include new API

2. **`src/config.rs`**
   - **REMOVE**: `ServerConfig` struct completely
   - **REMOVE**: `ConfigError` enum completely
   - File can be deleted if no other content remains

3. **`scripts/run_local_server.rs`**
   - **REPLACE**: Complete rewrite to use new `DpAuthServer` API
   - Remove all references to old `ServerConfig`

4. **`README.md`**
   - **REPLACE**: All usage examples with new API
   - **REMOVE**: All references to old `start_server` function
   - Update installation and configuration sections

5. **`tests/integration_tests.rs`**
   - **MIGRATE**: Move appropriate server functionality tests to `src/dp_auth_server.rs` as unit tests
   - **UPDATE**: Remaining integration tests to use new API
   - Focus on end-to-end testing rather than server internals

## Testing Strategy

1. **Builder Unit Tests** (`src/dp_auth_server_builder.rs`)
   - Test each builder method sets the correct field
   - Test `build()` method with valid configurations
   - Test `build()` method validation (missing required fields, invalid secret length)
   - Test default values are applied correctly
   - Test error handling for invalid configurations

2. **Server Unit Tests** (`src/dp_auth_server.rs`)
   - Test server initialization methods individually
   - Test logging initialization
   - Test database connection setup
   - Test GraphQL schema creation
   - Test router building with different configurations
   - Test listener binding
   - Test graceful shutdown handler setup
   - **Migrate appropriate tests from `tests/integration_tests.rs`** that test server internals

3. **Integration Tests** (`tests/integration_tests.rs`)
   - **Update existing tests** to use new `DpAuthServer` API
   - Test complete server startup and shutdown cycle
   - Test end-to-end API functionality
   - Focus on external behavior rather than internal implementation
   - **Remove tests** that are better suited as unit tests in the server module

4. **Breaking Change Tests**
   - Ensure old API is completely removed
   - Verify compilation fails with old usage patterns
   - Test that all public exports work correctly

## Benefits

1. **Better Encapsulation**: Server logic is contained within the `DpAuthServer` struct
2. **Cleaner API**: Builder pattern provides a more intuitive configuration experience
3. **Flexibility**: Optional parameters with sensible defaults
4. **Maintainability**: Server logic is broken down into smaller, focused methods
5. **Extensibility**: Easy to add new configuration options or server methods
6. **Simplified Codebase**: Removal of old API reduces maintenance burden

## Breaking Changes

This is a **major version bump** with complete breaking changes:

1. **Removed Functions**:
   - `start_server()` function
   - All exports related to old API

2. **Removed Structs**:
   - `ServerConfig` struct
   - `ConfigError` enum

3. **New Required Usage**:
   - Must use `DpAuthServer::new().build()?.start().await`
   - Must use builder pattern for configuration

4. **Migration Required**:
   - All existing code must be updated to use new API
   - No backward compatibility provided