# Plan: Fix RoleError Placement and Permission Check Return Type

## Overview
This plan fixes the architectural inconsistency where `RoleError` is incorrectly placed in the orchestrator layer instead of the service layer, following the `SiteService` pattern. The main issue is that `UserRoleService::check_user_permission` returns `sqlx::Error` instead of a proper service-level error type, which breaks the established patterns.

## Problem Analysis

### Current Issues
1. **RoleError is in the wrong layer**: `RoleError` exists in `src/orchestrators/role_orchestrator.rs` but should be in `src/services/user_role_service.rs` following the `SiteService` pattern
2. **Wrong return type for permission checks**: `UserRoleService::check_user_permission` returns `Result<bool, sqlx::Error>` but should return `Result<bool, RoleError>`
3. **Inconsistent error handling**: All orchestrators calling `UserRoleService::check_user_permission` expect `sqlx::Error` but should handle `RoleError`

### Target Pattern
Following `SiteService` in `src/services/site_service.rs`:
- Service has its own error enum with business logic errors
- Service methods return the service-specific error type
- Orchestrators handle service errors and convert them to orchestrator errors if needed

## Implementation Steps

### Step 1: Move RoleError to UserRoleService
- Move `RoleError` enum from `src/orchestrators/role_orchestrator.rs` to `src/services/user_role_service.rs`
- Follow the exact pattern from `SiteError` in `src/services/site_service.rs`
- Add missing error variants that will be needed for future role management:
  - `RoleNameAlreadyExists(String)`
  - `RoleInUse(i64)`
  - `ValidationError(String)`
  - `InvalidPermission(String)`

### Step 2: Update UserRoleService::check_user_permission Return Type
- Change `UserRoleService::check_user_permission` to return `Result<bool, RoleError>` instead of `Result<bool, sqlx::Error>`
- Update all database error handling to use `RoleError::DatabaseError(sqlx::Error)`
- Update the method implementation to handle the new error type

### Step 3: Update All UserRoleService Methods
- Update all existing `UserRoleService` methods to return `RoleError` instead of `sqlx::Error`
- Methods to update:
  - `get_all_roles()` - return `Result<Vec<UserRole>, RoleError>`
  - `get_role_by_id()` - return `Result<Option<UserRole>, RoleError>`
  - `get_role_by_name()` - return `Result<Option<UserRole>, RoleError>`

### Step 4: Update RoleOrchestrator
- Remove `RoleError` definition from `src/orchestrators/role_orchestrator.rs`
- Import `RoleError` from `crate::services::user_role_service::RoleError`
- Update all orchestrator methods to handle `RoleError` from service calls
- Add `From<RoleError>` conversion for orchestrator errors if needed

### Step 5: Update All Other Orchestrators
Update all orchestrators that call `UserRoleService::check_user_permission`:

#### Files to Update:
- `src/orchestrators/site_orchestrator.rs`
- `src/orchestrators/user_orchestrator.rs`
- `src/graphql/resolvers/role_permissions.rs`

#### Changes Required:
- Import `RoleError` from `crate::services::user_role_service::RoleError`
- Update `UserRoleService::check_user_permission` calls to handle `RoleError`
- Convert `RoleError` to orchestrator-specific errors using `?` operator and `From` traits

### Step 6: Update All Tests
Update test files that call `UserRoleService::check_user_permission`:

#### Test Files to Update:
- `src/services/user_role_service.rs` (internal tests)
- `src/orchestrators/role_orchestrator.rs` (orchestrator tests)

#### Changes Required:
- Update test assertions to handle `RoleError` instead of `sqlx::Error`
- Update error matching patterns in tests

### Step 7: Verify No Other Files Need Updates
Search the codebase to ensure no other files are directly calling `UserRoleService::check_user_permission` or importing the old `RoleError` location.

## Detailed Changes

### New RoleError Enum Structure
```rust
#[derive(Debug)]
pub enum RoleError {
  /// Database operation failed
  DatabaseError(sqlx::Error),
  /// Authentication failed (used by orchestrators)
  AuthenticationError(String),
  /// Authorization failed (used by orchestrators)
  AuthorizationError(String),
  /// Role not found
  RoleNotFound(i64),
  /// Role name already exists
  RoleNameAlreadyExists(String),
  /// Role is in use and cannot be deleted
  RoleInUse(i64),
  /// Input validation failed
  ValidationError(String),
  /// Invalid permission string
  InvalidPermission(String),
}
```

### Updated Method Signatures
```rust
// Before
pub async fn check_user_permission(
  pool: &SqlitePool,
  user: &User,
  permission: &str,
) -> Result<bool, sqlx::Error>

// After
pub async fn check_user_permission(
  pool: &SqlitePool,
  user: &User,
  permission: &str,
) -> Result<bool, RoleError>
```

### Error Conversion in Orchestrators
```rust
// In orchestrators, convert service errors to orchestrator errors
impl From<RoleError> for SiteOrchestratorError {
  fn from(err: RoleError) -> Self {
    match err {
      RoleError::DatabaseError(db_err) => SiteOrchestratorError::DatabaseError(db_err),
      RoleError::AuthenticationError(msg) => SiteOrchestratorError::AuthenticationError(msg),
      RoleError::AuthorizationError(msg) => SiteOrchestratorError::AuthorizationError(msg),
      // Handle other RoleError variants as needed
    }
  }
}
```

## Files Affected

### Modified Files
1. `src/services/user_role_service.rs`
   - Add `RoleError` enum
   - Update all method return types
   - Update all method implementations
   - Update all tests

2. `src/orchestrators/role_orchestrator.rs`
   - Remove `RoleError` definition
   - Import `RoleError` from service
   - Update all method implementations
   - Update all tests

3. `src/orchestrators/site_orchestrator.rs`
   - Import `RoleError` from service
   - Update `UserRoleService::check_user_permission` calls
   - Add error conversion if needed

4. `src/orchestrators/user_orchestrator.rs`
   - Import `RoleError` from service
   - Update `UserRoleService::check_user_permission` calls
   - Add error conversion if needed

5. `src/graphql/resolvers/role_permissions.rs`
   - Import `RoleError` from service
   - Update `UserRoleService::check_user_permission` calls

## Testing Strategy

### Unit Tests
- Test all `UserRoleService` methods with new `RoleError` return types
- Test error conversion and handling
- Verify all existing functionality still works

### Integration Tests
- Test all orchestrator methods with updated error handling
- Test permission checking with new error types
- Verify error propagation works correctly

### Regression Tests
- Run all existing tests to ensure no functionality is broken
- Verify all GraphQL resolvers still work correctly
- Test all permission checking scenarios

## Implementation Notes

### Error Handling Philosophy
- Service layer: Business logic errors + database errors wrapped in service error type
- Orchestrator layer: Authentication/authorization errors + converted service errors
- GraphQL layer: User-friendly error messages from orchestrator errors

### Backward Compatibility
- This is a breaking change for any code directly calling `UserRoleService::check_user_permission`
- However, since this is pre-1.0.0, breaking changes are acceptable for architectural consistency

### Future Considerations
- The new `RoleError` variants will be useful for the upcoming role management resolvers
- This change makes the codebase more consistent and maintainable
- Error handling will be more predictable across all service layers

## Validation Steps

1. **Compile Check**: Ensure all files compile after changes
2. **Unit Tests**: Run `cargo test --quiet` for all unit tests
3. **Integration Tests**: Run integration tests to verify orchestrator functionality
4. **Manual Testing**: Test GraphQL operations that depend on permission checking
5. **Error Scenarios**: Test various error conditions to ensure proper error handling

This plan ensures architectural consistency with the `SiteService` pattern while maintaining all existing functionality and improving error handling throughout the codebase.

## Suggested AGENTS.md Updates

### Add to "Code Style Guidelines" Section

#### Error Handling Architecture
- **Service Layer Error Types**: Each service must have its own error enum (e.g., `SiteError`, `UserError`, `RoleError`)
- **Service Error Placement**: Service error enums MUST be defined in their respective service files, NOT in orchestrator files
- **Service Method Return Types**: All service methods must return their service-specific error type, NOT `sqlx::Error` directly
- **Error Conversion**: Orchestrators must handle service errors and convert them to orchestrator-specific errors using `From` traits

#### Service Pattern Requirements
When creating new services, follow this exact pattern:

1. **Define Service Error Enum** in the service file:
   ```rust
   #[derive(Debug)]
   pub enum ServiceError {
     DatabaseError(sqlx::Error),
     ValidationError(String),
     // Add business logic-specific errors
   }
   
   impl std::fmt::Display for ServiceError { /* ... */ }
   impl std::error::Error for ServiceError {}
   impl From<sqlx::Error> for ServiceError { /* ... */ }
   ```

2. **Service Method Signatures** must return the service error type:
   ```rust
   pub async fn method_name(pool: &SqlitePool, params: Params) -> Result<ReturnType, ServiceError>
   ```

3. **Orchestrator Error Handling**:
   - Import service error type
   - Convert service errors to orchestrator errors using `From` traits
   - Never let `sqlx::Error` bubble up from service layer

#### Permission Check Pattern
- `UserRoleService::check_user_permission` MUST return `Result<bool, RoleError>`
- All orchestrators calling this method must handle `RoleError`
- Never return `sqlx::Error` from service methods

#### Validation Checklist
Before implementing new services or resolvers, verify:
- [ ] Service error enum is defined in service file (not orchestrator)
- [ ] All service methods return service-specific error type
- [ ] Permission checks return `RoleError`, not `sqlx::Error`
- [ ] Orchestrators properly convert service errors
- [ ] Tests handle the correct error types

This prevents future architectural inconsistencies where error types are placed in the wrong layer or methods return incorrect error types.