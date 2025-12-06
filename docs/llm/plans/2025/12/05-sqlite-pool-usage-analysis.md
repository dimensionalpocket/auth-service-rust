# SQLite Pool Usage Analysis in Tests

## Date
2025-12-05@11:45

## Overview
This document analyzes how SQLite connection pools are used across the test suite in the dps-auth-api project, focusing on pool ownership patterns, multiple usage within tests, and potential improvements.

## Current Pool Creation Patterns

### 1. Test Utility Function (`src/database/mod.rs:360-386`)
```rust
pub async fn create_test_database() -> (SqlitePool, NamedTempFile) {
    create_test_database_with_config(true).await
}

pub async fn create_test_database_with_config(
    configure_sqlite: bool,
) -> (SqlitePool, NamedTempFile) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new(&sqlite_file_path)
        .await
        .expect("Failed to create test database");
    let pool = database.pool;

    // Optionally configure SQLite settings
    if configure_sqlite {
        Database::configure_sqlite(&pool)
            .await
            .expect("Failed to configure SQLite");
    }

    // Run migrations
    let database = Database { pool: pool.clone() };
    database.migrate().await.expect("Failed to run migrations");

    (pool, temp_file)
}
```

### 2. Integration Test Pattern (`tests/integration_tests.rs:12-43`)
```rust
async fn create_app() -> Router {
    let temp_file = tempfile::Builder::new()
        .prefix(&format!("dps_auth_api_test_{}_", rand::random::<u32>()))
        .suffix(".db")
        .tempfile()
        .expect("Failed to create temp file");
    let db_path = temp_file.path().to_str().expect("Failed to get temp file path");

    let mut config = DpsConfig::new();
    config.set_auth_api_sqlite_main_pool_size(Some(1)); // Small pool for tests
    
    let server = DpsAuthApi::new(config).unwrap();
    server.migrate_database().await.expect("Failed to run migrations");
    server.seed_database().await.expect("Failed to run seeds");
    
    server.create_app().await.unwrap()
}
```

## Pool Usage Analysis by Test Type

### 1. Unit Tests (Services, Queries, Orchestrators, Resolvers)

**Pattern**: Single pool per test
- **Ownership**: Each test creates its own pool via `create_test_database()`
- **Lifetime**: Pool exists for the duration of the test function
- **Cleanup**: TempFile is dropped automatically, cleaning up the database file

**Files with Unit Tests Using `create_test_database()`**:

#### Services (`src/services/`)
- ✅ `user_service.rs` (15 tests) - Lines: 292, 317, 344, 455, 480, 489, 517, 545, 556, 599, 634, 669, 698, 722, 750, 759
- ✅ `auth_service.rs` (3 tests) - Lines: 201, 221, 237
- ✅ `site_service.rs` (9 tests) - Lines: 215, 238, 338, 368, 410, 451, 472, 501, 533
- ✅ `session_service.rs` (7 tests) - Lines: 188, 208, 218, 228, 241, 252, 273
- ✅ `user_role_service.rs` (5 tests) - Lines: 70, 91, 111, 152, 200
- ❌ `server_service.rs` - Has tests but doesn't use database (tests shutdown functionality)
- ❌ `password_service.rs` - Has tests but doesn't use database (tests password hashing)
- ❌ `shutdown_service.rs` - No tests found

#### Queries (`src/queries/`)
**Users Queries**:
- ✅ `delete_user_by_id.rs` (3 tests) - Lines: 25, 74, 83
- ✅ `get_user_by_id_with_role.rs` (4 tests) - Lines: 82, 101, 111, 138
- ✅ `get_all_users_with_roles.rs` (3 tests) - Lines: 30, 38, 74
- ✅ `update_user_password.rs` (3 tests) - Lines: 75, 110, 125
- ✅ `create_user.rs` (6 tests) - Lines: 72, 105, 143, 160, 188, 224
- ✅ `get_user_by_name.rs` (3 tests) - Lines: 25, 60, 98
- ✅ `get_user_by_uuid.rs` (2 tests) - Lines: 25, 60
- ✅ `get_user_by_id.rs` (2 tests) - Lines: 25, 62

**Sites Queries**:
- ✅ `get_site_by_id.rs` (5 tests) - Lines: 28, 61, 71, 81, 128
- ✅ `delete_site.rs` (2 tests) - Lines: 39, 76
- ✅ `update_site.rs` (4 tests) - Lines: 104, 146, 190, 207
- ✅ `get_all_sites.rs` (2 tests) - Lines: 25, 32
- ✅ `create_site.rs` (3 tests) - Lines: 54, 77, 103

**User Roles Queries**:
- ✅ `get_role_by_name.rs` (3 tests) - Lines: 24, 45, 54
- ✅ `get_role_by_id.rs` (2 tests) - Lines: 24, 47
- ✅ `get_all_roles.rs` (2 tests) - Lines: 23, 42
- ✅ `get_default_user_role.rs` (3 tests) - Lines: 23, 41, 56

#### GraphQL Resolvers (`src/graphql/resolvers/`)
- ✅ `user.rs` (4 tests) - Lines: 98, 184, 233, 260
- ✅ `sites.rs` (2 tests) - Lines: 71, 89
- ✅ `users.rs` (5 tests) - Lines: 128, 182, 201, 231, 263
- ✅ `site.rs` (4 tests) - Lines: 105, 187, 236, 263
- ✅ `auth_login.rs` (2 tests) - Lines: 108, 172
- ✅ `auth_change_password.rs` (5 tests) - Lines: 140, 188, 226, 262, 297
- ✅ `auth_register.rs` (4 tests) - Lines: 113, 162, 198, 221
- ✅ `update_site.rs` (5 tests) - Lines: 157, 244, 293, 320, 369
- ✅ `remove_site.rs` (4 tests) - Lines: 120, 200, 249, 276
- ✅ `add_site.rs` (5 tests) - Lines: 141, 216, 265, 292, 346
- ✅ `auth_me.rs` (4 tests) - Lines: 152, 200, 219
- ❌ `auth_logout.rs` - No tests found
- ❌ `get_server_timestamp.rs` - No tests found

#### Orchestrators (`src/orchestrators/`)
- ✅ `auth_orchestrator.rs` (6 tests) - Lines: 86, 119, 145, 176, 211, 246
- ✅ `site_orchestrator.rs` (10 tests) - Lines: 183, 220, 248, 281, 318, 354, 393, 447, 500, 540
- ✅ `user_orchestrator.rs` (16 tests) - Lines: 193, 230, 249, 279, 303, 328, 363, 386, 420, 452, 484, 514, 548, 571, 606, 642, 674

**Multiple Pool Usage Within Tests**: 
❌ **None found** - Each test uses only one pool instance

**Same Pool Multiple Usage Analysis**:
✅ **All tests correctly use the same pool multiple times without ownership issues**

**Examples of Proper Multiple Pool Usage**:
- **Services**: `src/services/user_service.rs:317-333` - Uses same `&pool` for setup, user creation, and verification
- **Queries**: `src/queries/users/get_user_by_id_with_role.rs:111-125` - Uses same `&pool` to create multiple users and query them
- **Resolvers**: `src/graphql/resolvers/auth_login.rs:108-135` - Uses same `&pool` for setup and GraphQL schema creation
- **Orchestrators**: `src/orchestrators/auth_orchestrator.rs:86-114` - Uses same `&pool` for role creation, user creation, and orchestration

**Ownership Pattern**:
- Pool is created once: `let (pool, _temp_file) = create_test_database().await;`
- Pool is passed by reference (`&pool`) to all functions within the test
- No cloning or moving of the pool occurs in tests
- Clean ownership maintained throughout test execution

**Arc and Pool Architecture**:
- **SqlitePool**: Already implements `Clone` internally (sqlx provides this)
- **No Arc Needed**: Tests don't need `Arc<SqlitePool>` because:
  - Tests are single-threaded within each test function
  - Pool is owned by the test function and passed by reference
  - No shared ownership across threads is required
- **Production Usage**: In production, `SqlitePool` can be cloned directly when needed:
  - `SqlitePool` uses internal reference counting for connection management
  - Cloning a pool creates a new handle to the same underlying connection pool
  - No `Arc` wrapper needed - `SqlitePool` is already designed for sharing
- **Test Simplicity**: Tests use direct `&SqlitePool` references for simplicity and clarity

### 2. Database Integration Tests (`tests/database_integration_tests.rs`)

**Pattern**: Single pool per test using consolidated utility
```rust
use crate::database::test_utils::create_test_database;

#[tokio::test]
async fn test_complete_user_creation_flow() {
    let (pool, _temp_file) = create_test_database().await;
    // Test logic using pool
}
```

**Multiple Pool Usage Within Tests**: 
❌ **None found** - Each test uses only one pool

### 3. Integration Tests (`tests/integration_tests.rs`)

**Pattern**: No direct pool access - uses full app with embedded pool
- Tests use HTTP requests to the app, pool is managed internally
- Pool size explicitly set to 1 for tests: `config.set_auth_api_sqlite_main_pool_size(Some(1))`

**Multiple Pool Usage Within Tests**: 
❌ **None found** - Tests use app-level abstraction

### 4. GET Query Support Tests (`tests/get_query_support_tests.rs`)

**Pattern**: Similar to integration tests - full app with embedded pool
- Each test creates its own app with its own pool
- Pool size set to 1: `config.set_auth_api_sqlite_main_pool_size(Some(1))`

**Multiple Pool Usage Within Tests**: 
❌ **None found**

### 5. Logging Tests (`tests/logging_tests.rs`)

**Pattern**: Same as integration tests - full app approach
- Each test creates its own app and pool
- Pool size set to 1

**Multiple Pool Usage Within Tests**: 
❌ **None found**

### 6. Playground Tests (`tests/playground_tests.rs`)

**Pattern**: Same as other integration tests
- Each test creates its own app and pool
- Pool size set to 1

**Multiple Pool Usage Within Tests**: 
❌ **None found**

## Key Findings

### 1. **No Multiple Pool Usage Within Tests**
- **Result**: No tests were found that use multiple pools within the same test
- **Implication**: No complex pool ownership or sharing issues to address

### 2. **Consistent Single-Pool Pattern**
- All unit/integration tests follow the same pattern: one pool per test
- Pool is created at the beginning and used throughout the test
- Cleanup is handled automatically by Rust's ownership system

### 3. **Two Main Approaches**

#### Approach A: Direct Pool Access (Unit Tests)
```rust
let (pool, _temp_file) = create_test_database().await;
// Use pool directly with services/queries
```

#### Approach B: App-Level Abstraction (Integration Tests)
```rust
let app = create_app().await;
// Use HTTP requests to test through the full stack
```

### 4. **Pool Size Configuration**
- Unit tests: Use default pool size (not explicitly set)
- Integration tests: Explicitly set to 1 connection
- Reasoning: Prevents connection contention in test environment

### 5. **Test Isolation**
- Each test gets its own database file (via `NamedTempFile`)
- No shared state between tests
- Automatic cleanup when temp file goes out of scope

## Pool Ownership Patterns

### 1. **Standard Pattern (95% of tests)**
```rust
#[tokio::test]
async fn test_something() {
    let (pool, _temp_file) = create_test_database().await;
    // Test logic using pool
    // _temp_file underscore indicates intentional non-use
    // TempFile cleanup happens automatically
}
```

### 2. **Integration Test Pattern**
```rust
async fn create_app() -> Router {
    // Create temp file and config
    let server = DpsAuthApi::new(config).unwrap();
    // Pool is owned by the server, not exposed to test
    server.create_app().await.unwrap()
}
```

## Potential Issues and Observations

### 1. **Code Duplication in Test Setup** (RESOLVED)
- ✅ Consolidated `create_test_database()` implementations into single utility
- `tests/database_integration_tests.rs` now uses shared utility from `src/database/mod.rs`
- `tests/integration_tests.rs` still has `create_app()` with similar logic (different pattern)

### 2. **Inconsistent Migration Handling**
- Unit test utility runs migrations automatically
- Integration tests call `server.migrate_database()` explicitly
- Some integration tests also call `server.seed_database()`

### 3. **Pool Configuration Variations**
- Unit tests: No explicit pool size (uses sqlx defaults)
- Integration tests: Explicitly set to 1
- This could lead to different behavior between test types

### 4. **Test Performance Considerations**
- Each test creates a new database and runs all migrations
- For large test suites, this could be slow
- No shared database or test transaction rollback pattern

## Recommendations

### 1. **Consolidate Test Utilities** (PARTIALLY COMPLETED)
- ✅ Consolidated `create_test_database()` implementations into single utility
- Added `create_test_database_with_config()` for flexible SQLite configuration
- Still need to consider standardizing integration test `create_app()` pattern
- Standardize migration and seeding behavior across test types

### 2. **Standardize Pool Configuration**
- Use consistent pool size across all test types
- Consider making pool size configurable for different test scenarios

### 3. **Consider Test Performance Optimizations**
- Implement test transaction rollback for faster tests
- Use in-memory SQLite for non-persistence tests
- Consider test database reuse patterns where appropriate

### 4. **Improve Test Documentation**
- Document the two main testing approaches and when to use each
- Add examples of proper pool usage patterns
- Clarify cleanup and isolation guarantees

## Quick Reference for Future Sessions

- **Pool Usage**: Exactly one SQLite pool per test - never multiple pools within single tests
- **Unit Tests**: Use direct pool access via `create_test_database()` from `src/database/mod.rs:360`
- **Configurable Tests**: Use `create_test_database_with_config(configure_sqlite: bool)` for optional SQLite configuration
- **Integration Tests**: Use full app with embedded pool via local `create_app()` functions:
  - `tests/integration_tests.rs:12` - main integration tests
  - `tests/logging_tests.rs:11` - logging-specific tests  
  - `tests/get_query_support_tests.rs` - uses `DpsAuthApi::create_app()` directly
- **Ownership**: Clean ownership with automatic temp file cleanup via `NamedTempFile` dropping
- **Pool Size**: Unit tests use defaults, integration tests set to 1 connection
- **No Multiple Pools**: No tests use multiple pools within the same test function
- **Same Pool Multiple Usage**: ✅ All tests correctly reuse same pool via `&pool` references without ownership issues
- **Arc Usage**: Not needed in tests - `SqlitePool` implements `Clone` internally and tests use single-threaded `&pool` references

## Conclusion

The current test suite demonstrates a clean, consistent approach to SQLite pool usage with no instances of multiple pools within single tests. The ownership patterns are clear and leverage Rust's ownership system for automatic cleanup. The main areas for improvement are around reducing code duplication and standardizing configuration across different test types.