# Consolidate Test Utilities Plan

**Created:** 2025-12-11@13:10  
**Status:** Planning

## Current State Analysis

### Existing `create_test_*` Methods Found

#### Database Test Utils (To Be Consolidated)
**Current File:** `src/database/mod.rs:354-398`  
**Methods to Move:**
- `create_test_database()` → `(SqlitePool, NamedTempFile)`
- `create_test_database_with_pool_size(pool_size: u32)` → `(SqlitePool, NamedTempFile)`
- `create_test_database_with_config(configure_sqlite: bool)` → `(SqlitePool, NamedTempFile)`
- `create_test_database_with_config_and_pool_size(configure_sqlite: bool, pool_size: u32)` → `(SqlitePool, NamedTempFile)`

**Target:** Move these to `src/test_utils/mod.rs` alongside the new consolidated methods

#### Duplicated Test Methods

**`create_test_user` Methods (8 implementations):**
- **Files:** `src/orchestrators/site_orchestrator.rs:153`, `src/orchestrators/user_orchestrator.rs:220`, `src/orchestrators/role_orchestrator.rs:193`, `src/graphql/resolvers/role_permissions.rs:69`, `src/queries/users/get_user_by_id_with_role.rs:46`, `src/orchestrators/auth_orchestrator.rs:54`, `src/graphql/resolvers/users.rs:92`, `src/graphql/resolvers/delete_user.rs:85`
- **Common Parameters:** `pool: &SqlitePool`, `username: &str`, `role_id: i64`
- **Returns:** `User` model
- **Implementation:** Uses `PasswordService::generate("password123")`, creates `CreateUserData` with UUID, calls `CreateUserQuery::run`

**`create_test_role` Methods (7 implementations):**
- **Files:** `src/orchestrators/site_orchestrator.rs:169`, `src/orchestrators/user_orchestrator.rs:233`, `src/queries/roles/update_role.rs:75`, `src/orchestrators/role_orchestrator.rs:209`, `src/graphql/resolvers/role_permissions.rs:100`, `src/graphql/resolvers/update_role.rs:147`, `src/queries/users/get_user_by_id_with_role.rs:62`
- **Common Parameters:** `pool: &SqlitePool`, `name: &str`, `permissions: &[&str]` (some use `Vec<&str>`)
- **Returns:** `i64` (role ID) or `Role` model
- **Implementation Variations:**
  - Some use hardcoded timestamp `1234567890i64`, others use `chrono::Utc::now().timestamp()`
  - Some return `i64`, others return full `Role` model
  - Different SQL INSERT patterns but functionally equivalent

**`create_test_user_via_mutation` Methods (2 implementations):**
- **Files:** `tests/integration_tests.rs:94`, `tests/logging_tests.rs:48`
- **Parameters:** `app: &Router`, `username: &str`, `password: &str`
- **Returns:** `String` (UUID)
- **Implementation:** Makes GraphQL `authRegister` mutation call

## Proposed Solution

### 1. Create Centralized Test Utilities Module

**Location:** `src/test_utils/mod.rs` (following Rust conventions for test utilities)

**Rationale for Location:**
- Top-level `src/test_utils/` follows Rust project structure best practices
- Separate from `database/test_utils` which is specifically for database setup
- Easily accessible from all test modules via `crate::test_utils::*`
- Keeps test utilities organized and discoverable

### 2. Module Documentation

**Implementation Task:** Add the following block comment to the top of `src/test_utils/mod.rs`:

```rust
/*
 * Shared test utilities for creating test data.
 * 
 * This module provides centralized functions for creating test users, roles, and other entities.
 * All helpers use existing Query objects to ensure consistency with application behavior.
 * 
 * Do NOT create local create_test_* functions in test modules - use these shared utilities instead.
 */
```

### 3. Consolidated Method Signatures

```rust
// src/test_utils/mod.rs
#[cfg(any(test, feature = "test-utils"))]
use crate::models::{User, Role};
use crate::queries::users::CreateUserQuery;
use crate::queries::roles::CreateRoleQuery;
use crate::services::password_service::PasswordService;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Create a test user with default password
pub async fn create_test_user(
    pool: &SqlitePool,
    username: &str,
    role_id: i64,
) -> User

/// Create a test user with custom password
pub async fn create_test_user_with_password(
    pool: &SqlitePool,
    username: &str,
    role_id: i64,
    password: &str,
) -> User

/// Create a test user with all parameters
pub async fn create_test_user_full(
    pool: &SqlitePool,
    username: &str,
    role_id: Option<i64>,
    password: &str,
    metadata_json: Option<serde_json::Value>,
) -> User

/// Create a test role and return ID
pub async fn create_test_role(
    pool: &SqlitePool,
    name: &str,
    permissions: &[&str],
) -> i64

/// Create a test role and return full Role model
pub async fn create_test_role_model(
    pool: &SqlitePool,
    name: &str,
    permissions: &[&str],
    is_default: bool,
) -> Role

/// Create a test user via GraphQL mutation (for integration tests)
pub async fn create_test_user_via_mutation(
    app: &axum::Router,
    username: &str,
    password: &str,
) -> String

// All functions in this module are automatically excluded from production builds
// by the #[cfg(any(test, feature = "test-utils"))] attribute at the module level
```

### 3. Implementation Details

**Standardization on Query Objects:**
- All test helpers will use existing Query objects instead of raw SQL
- This ensures consistency with application behavior and schema protection
- Eliminates duplicate SQL logic and reduces maintenance burden

**User Creation Methods:**
- Use `PasswordService::generate()` for password hashing
- Generate UUID using `Uuid::new_v4()`
- Use `CreateUserQuery::run()` for database insertion (already standardized)
- Support both role_id and None (for users without roles)
- Default password: "password123" for consistency

**Role Creation Methods:**
- Use `CreateRoleQuery::run()` instead of raw SQL queries
- Convert `&[&str]` permissions to `Vec<String>` for `CreateRoleData`
- Return both ID-only and full Role model variants
- Default `is_default: false`
- Leverage existing well-tested `CreateRoleQuery` logic

**Integration Test Method:**
- Keep GraphQL mutation approach for integration tests
- Return UUID string for test assertions

### 4. Migration Strategy

**Phase 1: Move database test utils (No new methods yet) ✅ COMPLETED**
- ✅ Create `src/test_utils/mod.rs` with ONLY the existing database test utils from `src/database/mod.rs:354-398`
- ✅ Add the module documentation block explaining the purpose
- ✅ Wrap the entire module with `#[cfg(any(test, feature = "test-utils"))]` at the module level, not individual methods
- ✅ Update all imports from `crate::database::test_utils::*` to `crate::test_utils::*` (48 files updated)
- ✅ Remove the `test_utils` module from `src/database/mod.rs`
- ✅ **MANDATORY REQUIREMENT:** All tests must pass after this phase before proceeding (412 tests passed)

**Phase 2: Add new consolidated test methods**
- Add the new `create_test_user`, `create_test_role`, and `create_test_user_via_mutation` methods to `src/test_utils/mod.rs`
- Add comprehensive unit tests for the new utilities
- Note: No need for individual method annotations since the entire module is already wrapped

**Phase 3: Replace duplicated implementations**
- Replace all 17 duplicated method implementations with imports
- Update method calls to use centralized utilities
- Ensure all existing tests continue to pass

**Phase 4: Clean up**
- Remove all local `create_test_*` function definitions from individual files
- Run full test suite to verify no regressions
- Update AGENTS.md with test utilities documentation

### 5. Files to Modify

**New Files:**
- `src/test_utils/mod.rs` - Centralized test utilities

**Files to Update (Import Changes):**
- `src/orchestrators/site_orchestrator.rs`
- `src/orchestrators/user_orchestrator.rs`
- `src/orchestrators/role_orchestrator.rs`
- `src/orchestrators/auth_orchestrator.rs`
- `src/graphql/resolvers/role_permissions.rs`
- `src/graphql/resolvers/update_role.rs`
- `src/graphql/resolvers/users.rs`
- `src/graphql/resolvers/delete_user.rs`
- `src/queries/roles/update_role.rs`
- `src/queries/users/get_user_by_id_with_role.rs`
- `tests/integration_tests.rs`
- `tests/logging_tests.rs`
- **All files using `crate::database::test_utils::*`** - update imports to `crate::test_utils::*`

### 6. AGENTS.md Update

**Implementation Task:** Update AGENTS.md to add the following to the **Testing** section:

```
### Test Utilities

- Shared test utilities are available in `src/test_utils/mod.rs`
- Use `create_test_user()`, `create_test_role()`, and `create_test_user_via_mutation()` for creating test data
- Database setup utilities remain in `src/database/test_utils.rs`
- Do not create local `create_test_*` functions in test modules - use the shared utilities instead
```

### 7. Benefits

- **DRY Principle:** Eliminates 17 duplicate implementations
- **Consistency:** Standardized test data creation across all tests using Query objects
- **Maintainability:** Single source of truth for test utilities
- **Schema Protection:** Query objects handle schema changes automatically
- **Business Logic Preservation:** Test data behaves exactly like real application data
- **Flexibility:** Multiple method variants support different test needs
- **Reliability:** Centralized and well-tested utilities reduce test flakiness

### 8. Risk Mitigation

- All existing tests must continue to pass after migration
- Maintain backward compatibility during transition
- Comprehensive unit tests for the utilities themselves
- Gradual migration approach to minimize disruption