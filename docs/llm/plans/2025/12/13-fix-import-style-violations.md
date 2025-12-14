# Fix Import Style Violations

**Date**: 2025-12-13  
**Status**: Planning Phase  
**Type**: Code Style Refactoring  

## Updated Import Guidelines

Based on AGENTS.md, the current import rules are:

1. **Prefer specific imports over `use *;`**
2. **Never use full paths in function bodies** - always import first
3. **Exceptions**: Types named "Error" should use full paths in code (e.g., `sqlx::Error`) and don't need to be imported

## Violations Analysis Report

### Summary of Findings

**Scale**: ~15 violations across 4 main files
**Impact**: Low (mostly import organization, minimal functional impact)
**Good news**: No wildcard imports found, Error types already handled correctly

### 1. Full Path Violations (Primary Issue)

#### A. Service Layer Violations

**File**: `src/services/role_service.rs`

**Current violations**:
- **Line 104**: `if !crate::models::role::is_valid_role_permission(permission)`
- **Line 185**: `if !crate::models::role::is_valid_role_permission(permission)`
- **Line 252**: `if !crate::models::role::is_valid_role_permission(permission)`
- **Line 735**: `let all_permissions: Vec<String> = crate::models::ROLE_PERMISSIONS`
- **Line 914**: `let all_permissions: Vec<String> = crate::models::ROLE_PERMISSIONS`
- **Line 1015**: `let user_data = crate::queries::users::CreateUserData`
- **Line 1022**: `crate::queries::users::CreateUserQuery::run(&pool, user_data)`

**Required imports**:
```rust
use crate::models::role::is_valid_role_permission;
use crate::models::ROLE_PERMISSIONS;
use crate::queries::users::{CreateUserData, CreateUserQuery};
```

#### B. Test Utilities Violations

**File**: `src/test_utils/mod.rs`

**Current violations**:
- **Line 103**: `use crate::queries::roles::GetDefaultRoleQuery;` (inside function)
- **Line 259**: `use crate::models::ROLE_PERMISSIONS;` (inside function)
- **Line 273**: `use crate::queries::roles::GetDefaultRoleQuery;` (inside function)
- **Lines 400, 433, 479, 512**: `use crate::middleware::session::SessionContext;` (inside functions)

**Required imports**:
```rust
use crate::models::ROLE_PERMISSIONS;
use crate::queries::roles::{GetDefaultRoleQuery, GetRoleByIdQuery};
use crate::middleware::session::SessionContext;
```

#### C. GraphQL Resolvers Violations

**File**: `src/graphql/resolvers/set_default_role.rs`
- **Line 225**: `use crate::queries::roles::GetRoleByIdQuery;` (inside function)

**File**: `src/graphql/resolvers/auth_login.rs`
- **Line 87**: `use crate::services::UserService;` (inside function)

**Required imports**:
```rust
// set_default_role.rs
use crate::queries::roles::GetRoleByIdQuery;

// auth_login.rs  
use crate::services::UserService;
```

### 2. Generic Type Violations

**Status**: ✅ **No violations found**

The codebase correctly follows the Error type exception:
- `sqlx::Error` is properly used with full paths without importing
- Custom error types (`UserError`, `RoleError`, `SiteError`) are properly imported as project-specific types

### 3. Wildcard Imports

**Status**: ✅ **No violations found**

No `use *;` statements exist in the codebase.

## Implementation Plan

### Phase 1: Fix Service Layer (High Priority)

**Scope**: `src/services/role_service.rs`

**Steps**:
1. **Add missing imports** at file top:
   ```rust
   use crate::models::role::is_valid_role_permission;
   use crate::models::ROLE_PERMISSIONS;
   use crate::queries::users::{CreateUserData, CreateUserQuery};
   ```

2. **Replace full path usage** with direct usage:
   - `crate::models::role::is_valid_role_permission` → `is_valid_role_permission`
   - `crate::models::ROLE_PERMISSIONS` → `ROLE_PERMISSIONS`
   - `crate::queries::users::CreateUserData` → `CreateUserData`
   - `crate::queries::users::CreateUserQuery` → `CreateUserQuery`

### Phase 2: Fix Test Utilities (High Priority)

**Scope**: `src/test_utils/mod.rs`

**Steps**:
1. **Add missing imports** at file top:
   ```rust
   use crate::models::ROLE_PERMISSIONS;
   use crate::queries::roles::{GetDefaultRoleQuery, GetRoleByIdQuery};
   use crate::middleware::session::SessionContext;
   ```

2. **Remove function-level imports** and use the module-level ones

### Phase 3: Fix GraphQL Resolvers (Medium Priority)

**Scope**: Two resolver files

**Steps**:
1. **`src/graphql/resolvers/set_default_role.rs`**:
   - Add import: `use crate::queries::roles::GetRoleByIdQuery;`
   - Remove function-level import

2. **`src/graphql/resolvers/auth_login.rs`**:
   - Add import: `use crate::services::UserService;`
   - Remove function-level import

### Phase 4: Validation (Final)

**Steps**:
1. **Build verification**: `cargo build`
2. **Test execution**: `cargo test --quiet`  
3. **Lint check**: `cargo clippy --allow-dirty --fix && cargo fmt`

## Implementation Details

### Systematic Processing Approach

1. **Start with services** - core business logic, high impact
2. **Move to test utilities** - shared functionality, affects all tests
3. **Handle resolvers** - API layer, isolated changes
4. **Validate everything** - ensure no regressions

### Per-File Processing Strategy

For each file:
1. **Identify all `crate::` usages** in function bodies
2. **Group by module** (models, queries, services, etc.)
3. **Add organized imports** at file top
4. **Replace full paths** with direct usage
5. **Run `cargo check`** to verify compilation

### Import Organization

Follow this import order in each file:
1. **Standard library** (`std::*`)
2. **External crates** (`sqlx`, `async_graphql`, etc.)
3. **Internal modules** (`crate::models`, `crate::services`, etc.)

## Risk Mitigation

1. **Incremental validation**: Run `cargo check` after each file
2. **Test preservation**: All changes are import-only, no functional changes
3. **Backup protection**: Git provides rollback capability
4. **Simple scope**: Limited number of files, clear patterns

## Expected Outcomes

1. **Full compliance** with updated import guidelines
2. **Cleaner function bodies** without `crate::` full paths
3. **Organized imports** at module level
4. **Maintained functionality** with improved readability
5. **Consistent patterns** across the codebase

## Success Criteria

- [ ] No `crate::` usage in function bodies (excluding Error types)
- [ ] All types properly imported at module level
- [ ] Error types continue to use full paths without imports
- [ ] `cargo build` succeeds without errors
- [ ] `cargo test --quiet` passes all tests
- [ ] `cargo clippy --allow-dirty --fix && cargo fmt` runs cleanly
- [ ] No `use *;` imports (maintained)

## File Processing Order

### Phase 1:
1. `src/services/role_service.rs`

### Phase 2:
2. `src/test_utils/mod.rs`

### Phase 3:
3. `src/graphql/resolvers/set_default_role.rs`
4. `src/graphql/resolvers/auth_login.rs`

This focused plan addresses the specific import violations found in the codebase while maintaining the correct handling of Error types and avoiding any unnecessary changes to code that's already compliant.