# Database Connection Architecture Migration Plan

**Date**: 2025-12-16@11:20  
**Purpose**: Address concurrency issues by migrating from pool-based to connection-based architecture for services and queries

## Problem Statement

Current architecture has a race condition where services running multiple queries may get different connections from the pool, causing the second query to not see changes from the first query. This breaks transactional consistency.

## Proposed Solution

- **Resolvers & Orchestrators**: Continue receiving `&SqlitePool`
- **Services & Queries**: Receive `&mut sqlx::sqlite::SqliteConnection` instead of pool
- **Orchestrators**: New responsibility to acquire connection from pool and pass to services/queries

Connection acquisition pattern:
```rust
let mut conn = pool.acquire().await?;
```

## Current Architecture Analysis

### Service-to-Query Mapping

#### User Domain
- **UserService**: GetUserByNameQuery, GetUserByIdQuery, CreateUserQuery, UpdateUserPasswordQuery, UpdateUserQuery, DeleteUserByIdQuery
- **AuthService**: GetUserByNameWithRoleQuery, GetUserByIdWithRoleQuery  
- **UserOrchestrator**: GetUserByIdQuery, GetAllUsersWithRolesQuery, GetUserByIdWithRoleQuery

#### Role Domain
- **RoleService**: GetAllRolesQuery, GetRoleByIdQuery, GetRoleByNameQuery, CreateRoleQuery, DeleteRoleQuery, UpdateRoleQuery, SetDefaultRoleQuery
- **RoleOrchestrator**: GetUserByIdQuery, GetAllRolesQuery, GetRoleByIdQuery, GetRoleByNameQuery, CreateRoleQuery, DeleteRoleQuery, UpdateRoleQuery, SetDefaultRoleQuery

#### Site Domain  
- **SiteService**: CreateSiteQuery, GetAllSitesQuery, UpdateSiteQuery, DeleteSiteQuery, GetSiteByIdQuery
- **SiteOrchestrator**: GetUserByIdQuery, GetSiteByIdQuery

#### Auth Domain (Cross-domain)
- **AuthService**: User domain queries + SessionService
- **SessionService**: GetUserByNameQuery (User domain)

### Query Usage Classification

#### Single-Service Queries (Safe for early phases)
- CreateUserQuery (UserService only, but also used in test utilities)
- UpdateUserPasswordQuery (UserService only)  
- UpdateUserQuery (UserService only)
- DeleteUserByIdQuery (UserService only)
- CreateRoleQuery (RoleService only, but also used in test utilities)
- DeleteRoleQuery (RoleService only)
- UpdateRoleQuery (RoleService only)
- SetDefaultRoleQuery (RoleService only)
- CreateSiteQuery (SiteService only)
- UpdateSiteQuery (SiteService only)
- DeleteSiteQuery (SiteService only)
- GetAllSitesQuery (SiteService only)

#### Multi-Service Queries (Complex, require final phase)
- GetUserByIdQuery: UserService, AuthService, UserOrchestrator, RoleOrchestrator, SiteOrchestrator
- GetUserByNameQuery: UserService, SessionService
- GetAllRolesQuery: RoleService, RoleOrchestrator
- GetUserByNameWithRoleQuery: AuthService only (but used by multiple orchestrator methods)
- GetUserByIdWithRoleQuery: AuthService, UserOrchestrator
- GetRoleByIdQuery: RoleService, RoleOrchestrator
- GetRoleByNameQuery: RoleService, RoleOrchestrator
- GetAllUsersWithRolesQuery: UserOrchestrator only

#### Test-Only Query Dependencies
- GetDefaultRoleQuery: Used only in test utilities
- GetUserByUuidQuery: Used only in test utilities for verification

**Critical Note**: Queries used in test utilities (CreateUserQuery, CreateRoleQuery, GetDefaultRoleQuery, GetUserByUuidQuery) must be handled in Phase 0 since test utilities call queries directly.

## Implementation Phases

### Phase 1: Proof of Concept - Single Query/Service/Orchestrator Chain
**Target**: `DeleteUserByIdQuery` + `UserService::delete_user` + `UserOrchestrator::delete_user_with_permission_check`
**Rationale**: 
- Single service calling single query (UserService only)
- Single orchestrator calling single service (UserOrchestrator only)
- Clear input/output boundaries with simple parameters
- Isolated from other functionality
- Good test case for ownership/lifetime issues
- No test utility dependencies (unlike CreateUserQuery)
- Tests the full chain: resolver → orchestrator → service → query

**Implementation Steps**:
1. Update `DeleteUserByIdQuery::run()` signature from `pool: &SqlitePool` to `conn: &mut SqliteConnection`
2. Change SQL execution from `.execute(pool)` to `.execute(&mut *conn)` (dereferencing required)
3. Update `UserService::delete_user()` signature from `pool: &SqlitePool` to `conn: &mut SqliteConnection`
4. Update `UserOrchestrator::delete_user_with_permission_check()` to acquire connection: `let mut conn = pool.acquire().await?;`
5. Pass connection from orchestrator to service: `UserService::delete_user(&mut *conn, target_user_id)` (dereferencing required)
6. Pass connection from service to query: `DeleteUserByIdQuery::run(&mut *conn, user_id)` (dereferencing required)
7. Update all tests to acquire connections: `let mut conn = pool.acquire().await.unwrap();`
8. Update all tests to use dereferenced connections: `UserService::delete_user(&mut *conn, ...)`
9. Update orchestrator to only acquire connection for delete operation, not for auth/permission checks
10. Run full test suite to ensure no regressions
11. Document any ownership/lifetime issues encountered

**Key Learning from Implementation**:
- **Connection Dereferencing**: sqlx queries require `&mut *conn` (dereferenced) not `&mut conn`
- **Connection Lifecycle**: Connections are automatically freed when going out of scope, enabling pool size 1 tests
- **Selective Connection Acquisition**: In Phase 1, orchestrators only acquire connections for operations that need them (temporary workaround). When all services and queries are updated, orchestrators will be expected to acquire one connection and reuse it for all service and query calls within the same operation.
- **Test Pattern**: Tests must acquire connections and use proper scoping to avoid pool exhaustion
- **Import Path Corrections**: Connection-based changes may reveal missing imports in orchestrators
- **Database Error Testing**: Creating reliable database error tests with connection-based approach is challenging. Tests that simulate database errors will be deleted and addressed later as needed during implementation.

### Phase 2: All Delete Queries for Single-Service Queries ✅ COMPLETE
**Target**: All delete queries called by only one service and their calling orchestrators (excluding test utility dependencies)
**Queries**: DeleteRoleQuery, DeleteSiteQuery
**Rationale**: Complete all delete operations together since DeleteUserByIdQuery was successfully implemented in Phase 1

**Note**: DeleteUserByIdQuery completed in Phase 1. Other single-service queries will be handled in Phase 3.

**Phase 2 Status: ✅ COMPLETE**

**Successfully Implemented**: DeleteRoleQuery + DeleteSiteQuery + their services and orchestrators

**Key Learnings Applied from Phase 1**:
- Connection dereferencing pattern: `&mut *conn` required for sqlx queries
- Connection lifecycle: Proper scoping ensures connections are released back to pool
- Connection acquisition pattern: `let mut conn = pool.acquire().await?;` in orchestrators
- Test connection management: Proper scoping prevents pool timeout issues
- Import management: Added SqliteConnection imports where needed, cleaned up unused imports

**Implementation Summary**:
- DeleteRoleQuery and DeleteSiteQuery migrated to `&mut SqliteConnection`
- RoleService::delete_role and SiteService::delete_site updated to use connections
- RoleOrchestrator and SiteOrchestrator updated to acquire connections for delete operations
- All related tests updated with proper connection acquisition patterns
- All 457 tests passing with no regressions

**Implementation Steps**:
1. Update all target query signatures to use `&mut SqliteConnection`
2. Update all calling service method signatures to use `&mut SqliteConnection`
3. Update all orchestrators to acquire one connection per operation and reuse it for all service/query calls
4. Update service methods that call multiple queries to reuse the same connection
5. Update orchestrator methods that call multiple services to reuse the same connection where possible
6. Update all related tests to use proper connection acquisition patterns
7. Update any GraphQL resolvers that call the updated orchestrators
8. Run full test suite

**Services to Update**:
- RoleService: delete_role
- SiteService: delete_site

**Orchestrators to Update**:
- RoleOrchestrator: methods calling RoleService delete_role
- SiteOrchestrator: methods calling SiteService delete_site

**Connection Management Pattern**: 
- Orchestrators acquire one connection: `let mut conn = pool.acquire().await?;`
- Reuse same connection for all service calls within the same operation
- Services receive `&mut SqliteConnection` and pass to all queries
- Connection automatically freed when orchestrator method completes

### Phase 3: Comprehensive Migration - All Remaining Components (Critical Foundation)
**Target**: ALL remaining queries, services, orchestrators, and test utilities to use connection-based architecture

**Rationale**: Previous session attempted incremental migration but failed. We need to change everything at once to avoid partial state issues. This phase will migrate ALL remaining components from pool-based to connection-based architecture in a single comprehensive update.

**Complete Component Inventory**:

#### All Remaining Queries (20 total):
**User Queries (9)**:
- `src/queries/users/get_user_by_name.rs`
- `src/queries/users/get_all_users_with_roles.rs`
- `src/queries/users/get_user_by_id_with_role.rs`
- `src/queries/users/create_user.rs`
- `src/queries/users/get_user_by_name_with_role.rs`
- `src/queries/users/get_user_by_id.rs`
- `src/queries/users/get_user_by_uuid.rs`
- `src/queries/users/update_user_password.rs`
- `src/queries/users/update_user.rs`

**Role Queries (7)**:
- `src/queries/roles/create_role.rs`
- `src/queries/roles/get_default_role.rs`
- `src/queries/roles/get_role_by_name.rs`
- `src/queries/roles/get_role_by_id.rs`
- `src/queries/roles/get_all_roles.rs`
- `src/queries/roles/update_role.rs`
- `src/queries/roles/set_default_role.rs`

**Site Queries (4)**:
- `src/queries/sites/get_site_by_id.rs`
- `src/queries/sites/get_all_sites.rs`
- `src/queries/sites/create_site.rs`
- `src/queries/sites/update_site.rs`

#### All Remaining Services (5 total):
- `src/services/user_service.rs` - Multiple methods still use pool
- `src/services/session_service.rs` - Methods still use pool
- `src/services/auth_service.rs` - Methods still use pool
- `src/services/role_service.rs` - Multiple methods still use pool
- `src/services/site_service.rs` - Methods still use pool

#### All Remaining Orchestrators (4 total):
- `src/orchestrators/user_orchestrator.rs` - Multiple methods still pass pool
- `src/orchestrator/role_orchestrator.rs` - Multiple methods still pass pool
- `src/orchestrator/site_orchestrator.rs` - Multiple methods still pass pool
- `src/orchestrator/auth_orchestrator.rs` - Methods still pass pool

#### Test Utilities (1 file with multiple functions):
- `src/test_utils/mod.rs` - All `create_test_*` functions need connection parameter
- **Exception**: `create_test_database_*` methods should continue returning `SqlitePool` as they are the foundation for acquiring connections

**Implementation Strategy**: Change everything at once to avoid partial state issues

**Implementation Steps - Split into 3 Sequential Steps**:

### Step 1: Comprehensive Code Changes (No Testing)
**Goal**: Make ALL necessary code changes across the entire codebase in one pass

1. **Update ALL Query Signatures**:
   - Change all 20 query `run()` methods from `pool: &SqlitePool` to `conn: &mut SqliteConnection`
   - Update all SQL execution from `.execute(pool)` to `.execute(&mut *conn)`
   - Update all query unit tests to acquire connections: `let mut conn = pool.acquire().await.unwrap();` **ONCE per test, reuse throughout**

2. **Update ALL Service Methods**:
   - Change all service method signatures from `pool: &SqlitePool` to `conn: &mut SqliteConnection`
   - Update all service method calls to pass `&mut *conn` to queries
   - Update all service unit tests to use connections

3. **Update ALL Orchestrator Methods**:
   - Add connection acquisition: `let mut conn = pool.acquire().await?;` at start of each method
   - Change all service calls to pass `&mut *conn` instead of pool
   - Ensure one connection per orchestrator method, reused for all service calls
   - Update all orchestrator unit tests

4. **Update ALL Test Utilities**:
   - Change all `create_test_*` function signatures from `pool: &SqlitePool` to `conn: &mut SqliteConnection`
   - **Exception**: Keep `create_test_database_*` methods returning `SqlitePool` - they are the foundation for acquiring connections
   - Update all query calls in test utilities to use `&mut *conn`
   - Update all test utility unit tests to acquire connections first
   - Update all integration tests that use test utilities
   - **Critical Testing Pattern**: Tests should acquire only ONE connection per test and reuse it for ALL helpers, queries, and services within that test

5. **Update ALL GraphQL Resolver Tests**:
   - Update any resolver tests that call queries directly to use connections
   - Update any resolver tests that use test utilities to pass connections

6. **Update ALL Integration Tests**:
   - Update `tests/database_integration_tests.rs` to use connections
   - Update any other integration tests that call queries directly

**Step 1 Completion Criteria**: All code changes made, no compilation verification yet

### Step 2: Compilation Verification (No Testing)
**Goal**: Ensure all code compiles successfully without running tests

1. **Run Compilation Check**:
   - Execute `cargo build` to verify all code compiles
   - Do NOT run any tests yet - focus only on compilation issues

2. **Resolve Compilation Errors**:
   - Fix any missing imports (e.g., `sqlx::SqliteConnection`)
   - Resolve any type mismatches or lifetime issues
   - Fix any syntax errors from parameter changes
   - Address any missing dereferencing (`&mut *conn` vs `&mut conn`)

3. **Iterate Until Clean Compilation**:
   - Continue `cargo build` cycles until all compilation errors are resolved
   - Ensure all 457+ test files compile (even if they don't pass yet)

**Step 2 Completion Criteria**: `cargo build` succeeds with no errors or warnings

### Step 3: Iterative Test Fixing
**Goal**: Fix all failing tests one by one until entire test suite passes

1. **Run Tests**:
   - Use `cargo test --quiet` for initial run
   - Use `--no-capture` for better error output if needed
   - Focus on one failing test at a time by examining the first failure in output

2. **Fix Each Test Iteratively**:
   - Analyze the first failing test's error message
   - Identify the root cause (connection acquisition, parameter passing, etc.)
   - Apply the fix to the specific test and any related tests with same pattern
   - Re-run compilation check to ensure fix doesn't break compilation
   - Re-run tests to verify fix and move to next failure

3. **Common Test Issues to Expect**:
   - Tests not acquiring connections before calling test utilities
   - Tests passing pools instead of connections to helpers
   - Multiple connections being acquired in same test (should be one)
   - Connection lifetime/scoping issues
   - Missing dereferencing in test code

4. **Iterate Until All Tests Pass**:
   - Continue fixing one test at a time
   - Run full test suite periodically to check progress
   - Ensure no regressions in previously fixed tests

**Step 3 Completion Criteria**: All 457+ tests pass with `cargo test --quiet`

**Connection Management Pattern for ALL Components**:
- **Orchestrators**: `let mut conn = pool.acquire().await?;` at method start
- **Services**: Receive `&mut SqliteConnection` and pass `&mut *conn` to queries
- **Queries**: Receive `&mut SqliteConnection` and use `&mut *conn` for sqlx operations
- **Test Utilities**: Receive `&mut SqliteConnection` and pass `&mut *conn` to queries
- **Tests**: `let mut conn = pool.acquire().await.unwrap();` **ONCE per test, then reuse for ALL helpers, queries, and services within that test**
- **Database Creation**: `create_test_database_*` methods continue returning `SqlitePool` for connection acquisition

**Critical Test Files to Update**:
- `tests/database_integration_tests.rs`
- `src/test_utils/mod.rs` (all test functions)
- Unit tests in all 5 service files
- Unit tests in all 4 orchestrator files
- Unit tests in all 20 query files
- GraphQL resolver tests that bypass normal flow

**Success Criteria**:
- All 457+ tests pass with connection-based architecture
- No remaining `&SqlitePool` parameters in queries/services/orchestrators
- All test utilities work with connections
- Connection acquisition pattern consistent across all components
- No pool timeout or connection exhaustion issues
- **Testing Pattern**: All tests acquire exactly ONE connection and reuse it throughout the test

**Note**: Phases 4, 5, 6, and 7 have been consolidated into the comprehensive Phase 3 above. The "change everything at once" approach in Phase 3 covers all remaining queries, services, orchestrators, and test utilities in a single migration to avoid partial state issues.

## Testing Strategy

### Unit Testing
- Each phase must include comprehensive unit tests
- Test connection reuse within service methods that call multiple queries
- Test error handling with connection failures
- Verify transactional behavior where applicable

### Integration Testing  
- Test full request flows from GraphQL resolvers through orchestrators to services
- Verify that concurrent requests don't interfere with each other
- Test edge cases like connection pool exhaustion
- Confirm integration tests continue working unchanged (they use full application)

### Regression Testing
- Full test suite must pass after each phase
- Performance testing to ensure no degradation
- Concurrency testing to verify race condition resolution

### Test Infrastructure (Phase 3)
- Update all test utilities to work with connection-based queries
- Ensure query unit tests can acquire connections properly
- Verify service tests can create test data with new approach
- Test utility validation to ensure test data creation works correctly

## Risk Mitigation

### Ownership Issues
- Phase 1 will identify potential ownership/lifetime problems
- Connection borrowing patterns may need adjustment
- May need to use `Arc<Mutex<SqliteConnection>>` in complex cases

### Performance Impact  
- Connection acquisition overhead should be minimal
- Monitor connection pool usage patterns
- Consider connection reuse optimization in orchestrators

## Success Criteria

1. **Concurrency Issue Resolution**: No race conditions between sequential queries in the same service call
2. **Performance**: No significant performance degradation
3. **Test Coverage**: All existing tests pass plus new connection-specific tests
4. **Code Quality**: Clean separation of concerns maintained
5. **Documentation**: Updated patterns and examples for future development

## Timeline Estimate

- **Phase 1**: 1-2 days (including learning and documentation) ✅ COMPLETE
- **Phase 2**: 1-2 days (delete queries only) ✅ COMPLETE
- **Phase 3**: 5-8 days (comprehensive migration of ALL remaining components)
  - **Step 1**: 1-2 days (comprehensive code changes)
  - **Step 2**: 1-2 days (compilation verification and fixes)
  - **Step 3**: 2-4 days (iterative test fixing)
- **Testing & Refinement**: 1-2 days (final validation)

**Total Estimated**: 8-14 days

**Note**: Phase 3 is now split into 3 sequential steps to ensure systematic completion:
1. **Step 1**: Make all code changes at once (no testing)
2. **Step 2**: Verify compilation and fix build issues (no testing)
3. **Step 3**: Fix tests one by one until all pass

This approach prevents getting overwhelmed by trying to fix everything at once and ensures each step is completed before moving to the next.

## Phase 1 Status: ✅ COMPLETE

**Successfully Implemented**: DeleteUserByIdQuery + UserService::delete_user + UserOrchestrator::delete_user_with_permission_check

**Key Learnings Applied to Future Phases**:
- Connection dereferencing pattern: `&mut *conn` required for sqlx queries
- Connection lifecycle: Automatic freeing when scope ends enables pool size 1 tests
- Selective connection acquisition: Temporary workaround in Phase 1, standard pattern in future phases
- Test connection management: Proper scoping and acquisition patterns
- Import path corrections: Connection changes may reveal missing orchestrator imports

## Next Steps

1. Review and approve updated comprehensive Phase 3 plan
2. Begin Phase 3 implementation (comprehensive migration of ALL remaining components)
3. Apply connection management patterns learned from Phases 1-2
4. Complete entire migration in single Phase 3 to avoid partial state issues
5. All components will be connection-based after Phase 3 completion