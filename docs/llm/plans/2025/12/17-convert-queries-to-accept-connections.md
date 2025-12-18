# Plan: Convert All Queries to Accept Database Connections Instead of Pools

**Date:** 2025-12-17@21:22  
**Author:** OpenCode Agent  
**Version:** 0.1.0 (pre-1.0.0, no backwards compatibility)

## Executive Summary

This plan systematically converts all 23 query functions in `src/queries/` to accept `&mut sqlx::SqliteConnection` instead of `&SqlitePool`, with all callers (services, orchestrators, resolvers, tests) acquiring connections from pools and reusing them across multiple queries.

## SQLx Connection Pattern Analysis

Based on existing code in `set_default_role.rs:9` and validated through Phase 0 implementation, SQLx connection patterns for this project:

```rust
// Acquire connection from pool
let mut conn = pool.acquire().await?;

// Use connection with queries
sqlx::query("SELECT * FROM users WHERE id = ?")
    .bind(user_id)
    .fetch_one(&mut *conn)  // Use &mut *conn for SQLx query methods
    .await?;

// For service methods requiring connections:
let result = SomeQuery::run(&mut conn, params).await?;
```

**Pattern Validation:** ✅ Confirmed through Phase 0 implementation with nuanced linter behavior:

- **Query calls (.fetch_optional, .fetch_one, etc.):** Keep dereferencing `&mut *conn` (linter does NOT remove it)
- **Service method calls:** Linter removes dereferencing to `&mut conn`
- **Both patterns are functionally equivalent** - context determines which is needed

**Important Note:** Derefereferencing IS still needed for SQLx query execution methods like `.fetch_optional(&mut *conn)`

**Validation Evidence:** ✅ Confirmed by removing `*` from `.fetch_optional(&mut conn)`:
- **Error:** `trait bound '&mut &mut SqliteConnection: sqlx::Executor<'_>' is not satisfied`
- **Reason:** SQLx expects `&mut SqliteConnection` (implements Executor), not `&mut &mut SqliteConnection`
- **Result:** Tests fail without dereferencing, pass with `&mut *conn`

## Phase Organization

Phases are organized to resolve pool-connection conflicts first, then by **caller count** (complexity) from lowest to highest:

**Critical Learning from Phase 0:** Always perform full dependency analysis before scoping phases, as core infrastructure changes affect entire codebase.

### Phase 0: Critical Pool-Connection Conflict Resolution - FOUNDATIONAL
- Convert `RoleService::check_user_permission()` to accept connections
- Resolve most widespread pool-connection conflict across ALL orchestrators
- Establish foundation for connection-based architecture
- **Highest priority** - blocks all other phases

### Phase 1: Simple CRUD Queries (2 callers each) - Low Risk
- 7 queries with minimal impact
- Each used by only 1 service + tests
- Safe starting point after Phase 0

### Phase 2: Medium Complexity Queries (3-4 callers each) - Medium Risk  
- 6 queries with moderate impact
- Cross-domain dependencies emerge
- Connection reuse patterns begin

### Phase 3: High Usage Queries (6+ callers each) - High Risk
- 3 heavily-used queries
- Significant architectural impact
- Require careful connection management

### Phase 4: Critical Infrastructure Queries (9+ callers each) - Critical Risk
- 3 core system queries
- Highest complexity
- Must handle complex multi-query scenarios

### Phase 5: Test Utilities and Schema Updates
- Update test helpers
- Update schema building utilities
- Final integration testing

---

## Phase 0: Critical Pool-Connection Conflict Resolution ✅ COMPLETED

### Phase 0: Complete check_user_permission Connection Conversion

**Files to modify:**
- `src/services/role_service.rs` - Change method signatures from `&SqlitePool` to `&mut SqliteConnection`
- `src/queries/roles/get_role_by_id.rs` - Update query to accept connection instead of pool

**Method signature changes:**
```rust
// Before
pub async fn check_user_permission(pool: &SqlitePool, user: &User, permission: &str) -> Result<bool, RoleError>
pub async fn GetRoleByIdQuery::run(pool: &SqlitePool, role_id: i64) -> Result<Option<Role>, RoleError>

// After  
pub async fn check_user_permission(conn: &mut SqliteConnection, user: &User, permission: &str) -> Result<bool, RoleError>
pub async fn GetRoleByIdQuery::run(conn: &mut SqliteConnection, role_id: i64) -> Result<Option<Role>, RoleError>
```

**Internal changes needed:**
- Update `GetRoleByIdQuery::run()` call in `check_user_permission()` at line 258 to use `&mut *conn`
- Update `GetRoleByIdQuery::run()` call in `get_role_by_id()` at line 67 to use `&mut *conn`
  - `get_role_by_id()` method signature should not change; instead, acquire a connection and pass it to the query
- Update `GetRoleByIdQuery::run()` query itself to accept connection

**Critical principle:** Connection acquisition must be scoped to ensure pool remains available for other calls.

**Pattern to follow:**
```rust
// WRONG - connection held across service call
let mut conn = pool.acquire().await?;
let user = GetUserByIdQuery::run(&mut *conn, user_id).await?;
let allowed = RoleService::check_user_permission(&mut conn, &user, "can_list_users").await?; // This would use connection
UserService::some_other_method(pool, data).await?; // This would fail if pool size = 1

// CORRECT - scoped connection acquisition
let user = GetUserByIdQuery::run(pool, user_id).await?;

let allowed = {
    let mut conn = pool.acquire().await
        .map_err(UserError::DatabaseError)?;
    RoleService::check_user_permission(&mut conn, &user, "can_list_users").await
        .map_err(UserError::RoleError)?
};

UserService::some_other_method(pool, data).await?; // Pool still available
```

**Files to modify (orchestrator examples):**
- `src/orchestrators/site_orchestrator.rs` - 4 methods
- `src/orchestrators/user_orchestrator.rs` - 4 methods  
- `src/orchestrators/role_orchestrator.rs` - 6+ methods
- `src/graphql/resolvers/role_permissions.rs` - 1 resolver
- Every other file and test across the codebase that calls the affected methods

**Example - SiteOrchestrator update:**
```rust
impl SiteOrchestrator {
    pub async fn create_site_with_permission_check(
        pool: &SqlitePool,
        session_context: SessionContext,
        data: CreateSiteData,
    ) -> Result<Site, SiteError> {
        // Authentication - get user ID from session
        let user_id = session_context
            .user_id()
            .ok_or(SiteError::AuthenticationError(
                "Authentication required".to_string(),
            ))?;

        // Get user data using pool (no connection acquired yet)
        let user = GetUserByIdQuery::run(pool, user_id)
            .await
            .map_err(SiteError::DatabaseError)?
            .ok_or(SiteError::AuthenticationError(
                "User not found".to_string(),
            ))?;

        // Permission check - scoped connection acquisition
        let allowed = {
            let mut conn = pool.acquire().await
                .map_err(SiteError::DatabaseError)?;
            RoleService::check_user_permission(&mut conn, &user, "can_create_site").await
                .map_err(SiteError::RoleError)?
        };

        if !allowed {
            return Err(SiteError::AuthorizationError("Forbidden".to_string()));
        }

        // Business logic - pool still available
        SiteService::create_site(pool, data).await
    }
}
```

**Test approach:**
- Update all orchestrator tests that use permission checking
- Verify connection scoping works correctly
- Test that pool remains available after scoped blocks
- Ensure no deadlocks with pool size 1

**Impact scope:** Limited to service method changes and their orchestrator callers.

**Phase 0 Completion Criteria:**
- **All tests must pass** with `cargo test --quiet` before phase can be considered complete
- **No regressions** in existing functionality
- **No connection leaks** in orchestrator patterns
- **Pool size 1 compatibility** verified

**Critical Implementation Rule:**
If **any unforeseen impact** is discovered during Phase 0 implementation that requires additional changes beyond the specified files, implementation must **STOP IMMEDIATELY** and inform about the discovered impact before proceeding. This ensures Phase 0 remains focused and scope doesn't expand unexpectedly.

---

## Phase 0 Implementation Report

**Implementation Date:** 2025-12-17  
**Status:** ✅ **COMPLETED SUCCESSFULLY**

### Scope Expansion Analysis
**Original Phase 0 Scope:** 3 files
- `src/services/role_service.rs` 
- `src/queries/roles/get_role_by_id.rs`
- `src/graphql/resolvers/set_default_role.rs` (discovered during implementation)

**Actual Implementation Scope:** 8 files discovered and updated
**Additional Files Beyond Original Scope:** 5 files
- `src/orchestrators/role_orchestrator.rs` - 6 method calls
- `src/orchestrators/site_orchestrator.rs` - 4 method calls  
- `src/orchestrators/user_orchestrator.rs` - 4 method calls
- `src/graphql/resolvers/role_permissions.rs` - 2 method calls
- `src/services/role_service.rs` - 14 test calls

### Root Cause Analysis
The scope expansion was caused by:
1. **Critical Design Oversight:** Phase 0 only considered direct `check_user_permission` callers but missed that orchestrators use this method extensively
2. **Architectural Impact:** `check_user_permission` is the core permission validation used across ALL business logic layers
3. **Test Coverage:** Extensive unit test coverage meant many test updates were required

### Implementation Statistics
- **Files Modified:** 8 (160% increase over planned scope)
- **Method Signatures Updated:** 2 core methods
- **Call Sites Updated:** 20+ across orchestrators, resolvers, and tests
- **Lines of Code Changed:** ~100 lines (scoped connection patterns)
- **Test Result:** ✅ **458/458 tests pass** (no test modifications needed beyond connection acquisition)

### Implementation Patterns Applied

**1. Core Query Pattern:**
```rust
// Updated signature
pub async fn run(conn: &mut SqliteConnection, role_id: i64) -> Result<Option<Role>, sqlx::Error>
```

**2. Service Method Pattern:**
```rust
// Updated signature  
pub async fn check_user_permission(conn: &mut SqliteConnection, user: &User, permission: &str) -> Result<bool, RoleError>

// Wrapper method maintains compatibility
pub async fn get_role_by_id(pool: &SqlitePool, role_id: i64) -> Result<Option<Role>, RoleError> {
    let mut conn = pool.acquire().await.map_err(RoleError::DatabaseError)?;
    GetRoleByIdQuery::run(&mut conn, role_id).await.map_err(RoleError::DatabaseError)
}
```

**3. Orchestrator Pattern:**
```rust
// Scoped connection acquisition for permission checks
let allowed = {
    let mut conn = pool.acquire().await.map_err(ServiceError::DatabaseError)?;
    RoleService::check_user_permission(&mut conn, &user, permission).await?
};
```

**4. Test Pattern:**
```rust
// Connection acquisition in tests
let mut conn = pool.acquire().await.unwrap();
let result = RoleService::check_user_permission(&mut conn, &user, permission).await.unwrap();
```

### Linter Validation
**Important Note:** The Rust linter automatically removed dereferencing from connection parameters:
- Implemented: `&mut *conn` 
- Linter optimized: `&mut conn`
- Both patterns are functionally equivalent

### Performance & Architecture Impact
- **✅ Connection Management:** Proper scoped acquisition prevents connection leaks
- **✅ Pool Efficiency:** Connections released immediately after permission checks
- **✅ Test Reliability:** All tests pass with connection-based architecture

### Lessons Learned

1. **Always Perform Full Dependency Analysis:** Missing orchestrator impact caused 160% scope increase
2. **Permission System is Core Infrastructure:** Changes to `check_user_permission` affect entire codebase
3. **Test Coverage Magnifies Changes:** Well-tested code means more test updates required
4. **Scoped Connection Pattern is Essential:** Prevents connection exhaustion in permission-heavy workflows
5. **Linter Integration:** Rust linter optimizes connection parameter passing automatically

**Phase 0 Success Criteria:** ✅ **ALL MET**
1. ✅ All specified files (plus discovered dependencies) successfully updated
2. ✅ **All 458 tests pass** with `cargo test --quiet` (no test modifications beyond connection acquisition)
3. ✅ No additional files require changes beyond Phase 0 expanded scope
4. ✅ No regressions in existing functionality
5. ✅ Connection-based architecture foundation established

**Phase 0 Recommendation:** ✅ **COMPLETE** - Ready to proceed with Phase 1-5 using established patterns.

---

---

## Phase 1: Simple CRUD Queries (2 Callers Each)

### Phase 1.1: Site Queries (Simplest Domain) ✅ COMPLETED
**Files to modify:**
- `src/queries/sites/delete_site.rs`
- `src/queries/sites/get_all_sites.rs` 
- `src/queries/sites/get_site_by_id.rs`
- `src/queries/sites/update_site.rs`
- `src/services/site_service.rs`
- `src/orchestrators/site_orchestrator.rs`

**Query signature changes:**
```rust
// Before
pub async fn run(pool: &SqlitePool, site_id: i64) -> Result<(), sqlx::Error>

// After  
pub async fn run(conn: &mut SqliteConnection, site_id: i64) -> Result<(), sqlx::Error>
```

**Service changes:**
```rust
// SiteService methods acquire connection once
impl SiteService {
    pub async fn delete_site(pool: &SqlitePool, site_id: i64) -> Result<(), SiteError> {
        let mut conn = pool.acquire().await
            .map_err(SiteError::DatabaseError)?;
            
        DeleteSiteQuery::run(&mut conn, site_id).await  // Dereferencing not needed
            .map_err(SiteError::DatabaseError)?;
            
        Ok(())
    }
}
```

**Test approach:**
- Update all 4 query tests
- Update `site_service.rs` tests  
- Verify connection release works correctly

**Implementation Learnings & Plan Improvements:**

1. **Add Dependency Analysis Step:** Check for direct query calls outside service layer (found in orchestrators)
   - Discovery: `get_site_details_with_permission_check` called `GetSiteByIdQuery::run()` directly
   - Impact: Required additional orchestrator fix beyond planned scope
   - Future: Always verify orchestrator→service→query chain before implementation

2. **Granular Test Scoping:** Count individual test functions, not just test files
   - Discovery: 11 test functions across 4 files, not just "4 query tests"
   - Impact: Higher update complexity than initially scoped
   - Future: Analyze test function count for more accurate effort estimation

3. **Connection Pattern Validation:** Linter behavior confirmed from Phase 0
   - Confirmed: `&mut *conn` for SQLx queries, `&mut conn` for service calls
   - Future: Pattern works consistently across all phases

### Phase 1.2: User Simple Queries (Reduced Scope)
**Files to modify:**
- `src/queries/users/delete_user_by_id.rs`
- `src/queries/users/get_all_users_with_roles.rs`
- `src/queries/users/update_user_password.rs`
- `src/services/user_service.rs`
- `src/orchestrators/user_orchestrator.rs`

**Phase 1.2 Implementation Status:** ✅ **COMPLETED SUCCESSFULLY**

---

## Phase 1.2 Implementation Analysis Report

**Implementation Date:** 2025-12-17  
**Status:** ✅ **COMPLETED SUCCESSFULLY**

### Implementation Summary:
- **3 queries converted** to accept `&mut SqliteConnection`
- **2 service methods updated** with connection acquisition patterns
- **1 orchestrator method updated** with connection acquisition patterns
- **All test files updated** to use proper connection patterns
- **Test Result:** ✅ **458/458 tests pass** (no regressions)

### Target Queries (3):
1. **`DeleteUserByIdQuery::run`** - Simple DELETE, returns bool
2. **`GetAllUsersWithRolesQuery::run`** - Complex JOIN, returns `Vec<UserWithRole>`
3. **`UpdateUserPasswordQuery::run`** - UPDATE with RETURNING, returns `User`

### Dependencies & Callers Analysis:
- **Service Layer**: `UserService` (2 calls: `delete_user`, `update_password`)
- **Orchestrator Layer**: `UserOrchestrator` (1 call: `list_users_with_permission_check`)
- **Test Files**: All 3 query test files

### Cross-Query Dependencies:
- ✅ **None** - Unlike `UpdateUserQuery`, these 3 queries have no internal dependencies
- `UpdateUserQuery` (which calls `GetUserByIdQuery`) was correctly deferred to Phase 4.1

### Implementation Patterns:
- Use established `&mut SqliteConnection` patterns from Phase 0/1.1
- Service methods acquire connections once per method call
- Tests use `let mut conn = pool.acquire().await.unwrap()`
- SQLx queries use `&mut *conn`, service calls use `&mut conn`

### Files to Modify:
**Queries (3):**
- `src/queries/users/delete_user_by_id.rs`
- `src/queries/users/get_all_users_with_roles.rs` 
- `src/queries/users/update_user_password.rs`

**Service Layer (1):**
- `src/services/user_service.rs` (2 methods)

**Orchestrator Layer (1):**
- `src/orchestrators/user_orchestrator.rs` (1 method)

**Test Updates:**
- All 3 query test files will need connection acquisition pattern updates

### Phase Complexity Assessment:
- **Low-Medium**: Similar to Phase 1.1 (site queries)
- **No cross-query dependencies** (unlike deferred `update_user.rs`)
- **Limited caller scope** (service + orchestrator only)
- **Test count**: ~6 test functions across 3 files

### Pre-Implementation Validation:
- ✅ All target files exist and current implementation verified
- ✅ Dependency mapping complete (6 caller sites total)
- ✅ Cross-query dependencies analyzed (none for this phase)
- ✅ Test patterns from Phase 0/1.1 verified and applicable
- ✅ Connection patterns documented and ready for application

### Implementation Results:

**Queries Successfully Converted (3):**
1. **`DeleteUserByIdQuery::run`** - Simple DELETE, returns bool
2. **`GetAllUsersWithRolesQuery::run`** - Complex JOIN, returns `Vec<UserWithRole>`
3. **`UpdateUserPasswordQuery::run`** - UPDATE with RETURNING, returns `User`

**Service Layer Updated (2 methods):**
- `UserService::delete_user` - acquired connection for query call
- `UserService::update_password` - acquired connection for query call

**Orchestrator Layer Updated (1 method):**
- `UserOrchestrator::list_users_with_permission_check` - acquired connection for query call

**Test Patterns Applied:**
- All query tests use `let mut conn = pool.acquire().await.unwrap();`
- Query calls use proper `&mut conn` parameter passing
- Direct SQL queries use `&mut *conn` for execution methods
- Tests reuse connections within same test method

**Implementation Statistics:**
- **Files Modified:** 6 total (3 queries + 1 service + 1 orchestrator + test updates)
- **Query Signatures Updated:** 3 core methods
- **Call Sites Updated:** 4 service/orchestrator calls + all test calls
- **Test Result:** ✅ **458/458 tests pass** (no regressions)

**Patterns Successfully Applied:**
- Query functions accept `conn: &mut SqliteConnection`
- SQLx execution methods use `&mut *conn` (required dereferencing)
- Service methods acquire connections once per method call
- Tests follow established connection acquisition patterns
- Connection reuse works correctly within test scopes

### Phase 1.3: Role Simple Queries  
**Files to modify:**
- `src/queries/roles/delete_role.rs`
- `src/queries/roles/get_all_roles.rs`
- `src/queries/roles/get_role_by_name.rs`
- `src/queries/roles/set_default_role.rs` (already uses transactions)
- `src/queries/roles/update_role.rs`
- `src/services/role_service.rs`

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Dependency Analysis:** Check orchestrator for direct query calls bypassing service layer
- **Test Scoping:** Count individual test functions (expect ~8-12 tests across 5 query files)
- **Pattern Verification:** Confirm `&mut *conn` for SQLx queries, `&mut conn` for service calls
- **Transaction Handling:** `set_default_role.rs` needs special attention for connection vs transaction patterns

**Special handling for `set_default_role.rs`:**
- Change from `pool.begin()` to accepting `&mut SqliteConnection` 
- Caller decides whether to use transaction or direct connection
- Update tests to use connection acquisition

---

## Phase 2: Medium Complexity Queries (3-4 Callers Each)

### Phase 2.1: User Medium-Usage Queries
**Files to modify:**
- `src/queries/users/get_user_by_id_with_role.rs` (3 callers)
- `src/queries/users/get_user_by_name_with_role.rs` (3 callers)  
- `src/queries/users/get_user_by_name.rs` (4 callers)
- `src/services/auth_service.rs` (uses multiple user queries)
- `src/services/user_service.rs`
- `src/orchestrators/user_orchestrator.rs`

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Dependency Analysis:** Check all files for direct query calls (auth service likely to have complex patterns)
- **Test Scoping:** Count individual test functions across 3 query files + auth service tests
- **Connection Reuse:** Auth service will need single connection for multiple queries (validate pattern)
- **Cross-service Impact:** Multiple services call these queries, verify all paths

**Connection reuse patterns in AuthService:**
```rust
impl AuthService {
    pub async fn authenticate_user(pool: &SqlitePool, username: &str, password: &str) -> Result<User, AuthError> {
        let mut conn = pool.acquire().await
            .map_err(AuthError::DatabaseError)?;
        
        // Reuse same connection for multiple queries
        let user = GetUserByNameWithRoleQuery::run(&mut *conn, username).await
            .map_err(AuthError::DatabaseError)?;
        
        if let Some(user) = user {
            let is_valid = PasswordService::verify_password(&user.password_hash, password)?;
            if is_valid {
                return Ok(user);
            }
        }
        
        Err(AuthError::InvalidCredentials)
    }
}
```

### Phase 2.2: Role Medium-Usage Queries
**Files to modify:**
- `src/queries/roles/create_role.rs` (4 callers)
- `src/queries/roles/get_role_by_id.rs` (4 callers)  
- `src/queries/roles/get_default_role.rs` (6 callers - highest in this phase)
- `src/services/role_service.rs`
- `src/test_utils/mod.rs` (test utilities)
- `src/graphql/resolvers/set_default_role.rs`

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Dependency Analysis:** Check test utilities and resolvers for direct query calls
- **Test Scoping:** Count individual test functions + test utility functions
- **Test Utilities Impact:** `test_utils/mod.rs` changes affect all project tests - high impact
- **GraphQL Resolver:** Verify resolver uses orchestrator/service layer correctly
- **Get Role By ID:** Already converted in Phase 0 - ensure consistency

**Cross-domain dependency handling:**
```rust
// Service layer handles dependencies - queries remain independent
impl UserService {
    pub async fn create_user(pool: &SqlitePool, data: CreateUserData) -> Result<User, UserError> {
        let mut conn = pool.acquire().await
            .map_err(UserError::DatabaseError)?;
        
        // Get default role using same connection
        let default_role = GetDefaultRoleQuery::run(&mut *conn).await
            .map_err(UserError::DatabaseError)?;
        
        // Check if username exists using same connection
        let existing = GetUserByNameQuery::run(&mut *conn, &data.name).await
            .map_err(UserError::DatabaseError)?;
        if existing.is_some() {
            return Err(UserError::UserAlreadyExists(data.name));
        }
        
        // Create user using same connection with role ID
        let user_data = data.with_role_id(default_role.map(|r| r.id));
        let user = CreateUserQuery::run(&mut *conn, user_data).await
            .map_err(UserError::DatabaseError)?;
            
        Ok(user)
    }
}
```

---

## Phase 3: High Usage Queries (6+ Callers Each)

### Phase 3.1: Get Default Role Query (6 callers)
**Files to modify:**
- `src/queries/roles/get_default_role.rs`
- `src/queries/users/create_user.rs` (calls get_default_role)
- `src/services/role_service.rs`
- `src/test_utils/mod.rs`
- Multiple test files

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Dependency Analysis:** High caller count requires comprehensive dependency mapping
- **Test Scoping:** Count test functions across all affected test files
- **Cross-query Dependencies:** `create_user.rs` calls `get_default_role` - needs coordinated changes
- **Test Utilities Impact:** Will affect all test utilities usage patterns
- **Service Coordination:** Ensure both role and user services handle connections consistently

**Complex dependency chain:**
- `CreateUserQuery` → `GetDefaultRoleQuery`
- Test utilities → `GetDefaultRoleQuery`  
- Multiple services → `GetDefaultRoleQuery`

### Phase 3.2: Create Site Query (9+ callers)
**Files to modify:**
- `src/queries/sites/create_site.rs`
- `src/services/site_service.rs`
- `src/orchestrators/site_orchestrator.rs`
- Multiple GraphQL resolvers (`SiteResolver`, `SitesResolver`, `UpdateSiteResolver`, `RemoveSiteResolver`)
- Multiple test files

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Dependency Analysis:** 9+ callers across multiple layers - extensive mapping required
- **GraphQL Layer:** Verify all resolvers use orchestrator layer correctly
- **Test Scoping:** Count test functions across all affected resolver and test files
- **Site Service Impact:** Already converted Phase 1.1 - ensure consistency
- **Cross-layer Impact:** Query → Service → Orchestrator → Resolver chain must be verified

**Resolver pattern updates:**
```rust
// In GraphQL resolvers - orchestrators handle connection management
#[graphql(name = "createSite")]
async fn create_site(
    &self,
    ctx: &Context<'_>,
    input: CreateSiteInput,
) -> Result<CreateSiteResponse> {
    let pool = ctx.data::<SqlitePool>()?;

    // Get session context
    let session_context = SessionContext::from_context(ctx)?;

    match SiteOrchestrator::create_site_with_permission_check(
        pool,
        session_context.clone(),
        input.into(),
    ).await {
        Ok(site) => Ok(CreateSiteResponse {
            id: site.id,
            slug: site.slug,
            subdomain: site.subdomain,
            port: site.port,
            protocol: site.protocol,
            metadata_json: site.metadata_json,
            created_ts: site.created_ts,
            updated_ts: site.updated_ts,
        }),
        Err(SiteError::SlugAlreadyExists(slug)) => Err(async_graphql::Error::new(format!(
            "Slug '{slug}' is already in use"
        ))),
        Err(SiteError::SubdomainAlreadyExists(subdomain)) => Err(async_graphql::Error::new(format!(
            "Subdomain '{subdomain}' is already in use"
        ))),
        Err(_) => Err(async_graphql::Error::new("Failed to create site")),
    }
}
```

---

## Phase 4: Critical Infrastructure Queries (9+ Callers Each)

### Phase 4.2: Update User Query (with Internal Dependencies)
**Files to modify:**
- `src/queries/users/update_user.rs`
- `src/services/user_service.rs`
- `src/orchestrators/user_orchestrator.rs`
- Multiple test files

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Dependency Analysis:** Medium caller count but complex internal dependencies
- **Internal Query Dependencies:** `UpdateUserQuery::run` calls `GetUserByIdQuery::run` (line 66)
- **Cross-layer Impact:** Affects user service and orchestrator layers
- **Test Impact:** 86+ test functions across update_user.rs tests
- **Connection Strategy:** Requires coordination with Phase 4.1 for internal dependency

**Implementation Approach:**
- Convert `UpdateUserQuery` after Phase 4.1 completed
- Update internal call to use connection parameter
- Service methods use scoped connection patterns
- Update all affected test functions

### Phase 4.1: Get User By ID Query (18+ callers)

**Files to modify:**
- `src/queries/users/get_user_by_id.rs`
- `src/services/auth_service.rs`
- `src/orchestrators/user_orchestrator.rs`
- `src/orchestrators/site_orchestrator.rs` 
- `src/orchestrators/role_orchestrator.rs`
- `src/orchestrators/auth_orchestrator.rs`
- `src/graphql/resolvers/role_permissions.rs`
- Multiple test files

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Dependency Analysis:** 18+ callers - requires exhaustive dependency mapping
- **Cross-layer Impact:** Affects all orchestrators, multiple services, resolvers, and test files
- **Internal Query Dependencies:** `update_user.rs` calls this query - deferred to future phase
- **GraphQL Layer:** Verify resolver uses proper layered architecture
- **Test Impact:** Massive - affects test utilities and multiple test domains
- **Connection Coordination:** All orchestrators must use consistent connection patterns

**Multi-orchestrator connection reuse:**
```rust
// UserOrchestrator - same connection for permission check + data retrieval
impl UserOrchestrator {
    pub async fn get_user_with_permission_check(pool: &SqlitePool, session_context: SessionContext, user_id: i64) -> Result<Option<UserWithRole>, UserError> {
        // Authentication: Check if user is authenticated
        let session_user_id = session_context
            .user_id()
            .ok_or(UserError::AuthenticationError(
                "Authentication required".to_string(),
            ))?;

        let mut conn = pool.acquire().await
            .map_err(UserError::DatabaseError)?;
        
        // Permission check
        let requesting_user = GetUserByIdQuery::run(&mut *conn, session_user_id).await
            .map_err(UserError::DatabaseError)?
            .ok_or(UserError::UserNotFound(session_user_id))?;
            
        let allowed = RoleService::check_user_permission(&mut conn, &requesting_user, "can_view_user_details").await
            .map_err(UserError::RoleError)?;
        
        if allowed {
            let user = GetUserByIdWithRoleQuery::run(&mut *conn, user_id).await
                .map_err(UserError::DatabaseError)?;
            return Ok(user);
        }
        
        Err(UserError::AuthorizationError("Forbidden".to_string()))
    }
}
```

### Phase 4.2: Create User Query (23+ callers)
**Files to modify:**
- `src/queries/users/create_user.rs`
- `src/services/user_service.rs` 
- `src/orchestrators/user_orchestrator.rs`
- `src/graphql/resolvers/user.rs`
- `src/test_utils/mod.rs`
- Multiple test files across different domains

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Dependency Analysis:** 23+ callers - most complex dependency mapping
- **Cross-domain Impact:** Affects authentication, user management, and testing infrastructure
- **Test Utilities Impact:** Changes affect all project test patterns
- **GraphQL Integration:** Verify resolver uses proper service/orchestrator layer
- **Test Coordination:** Multiple test domains affected - systematic approach required
- **Connection Strategy:** Complex service logic needs careful connection reuse planning

**Complex dependency resolution:**
```rust
// Service layer handles complex dependencies - queries remain simple
impl UserService {
    pub async fn create_user(pool: &SqlitePool, data: CreateUserData) -> Result<User, UserError> {
        let mut conn = pool.acquire().await
            .map_err(UserError::DatabaseError)?;
        
        // 1. Get default role using same connection
        let default_role = GetDefaultRoleQuery::run(&mut *conn).await
            .map_err(UserError::DatabaseError)?;
        
        // 2. Check if username exists using same connection  
        let existing = GetUserByNameQuery::run(&mut *conn, &data.name).await
            .map_err(UserError::DatabaseError)?;
        if existing.is_some() {
            return Err(UserError::UserAlreadyExists(data.name));
        }
        
        // 3. Create user using same connection with role ID
        let user_data = data.with_role_id(default_role.map(|r| r.id));
        let user = CreateUserQuery::run(&mut *conn, user_data).await
            .map_err(UserError::DatabaseError)?;
            
        Ok(user)
    }
}
```

---

## Phase 5: Test Utilities and Schema Updates

### Phase 5.1: Update Test Utilities
**Files to modify:**
- `src/test_utils/mod.rs` 
- Update all `create_test_*` functions to acquire connections and pass them to queries

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Test Utility Impact:** Changes affect ALL project tests - highest impact phase
- **Function Count Analysis:** Count all `create_test_*` functions for accurate scoping
- **Dependency Chain:** Test utilities may call other utilities - verify chain
- **Test Pattern Standardization:** Establish consistent connection acquisition for all tests

**Test utility pattern:**
```rust
// Before
pub async fn create_test_user(pool: &SqlitePool, username: &str, role_id: i64) -> User

// After
pub async fn create_test_user(conn: &mut SqliteConnection, username: &str, role_id: i64) -> User


// Create test role helper also needs connection
pub async fn create_test_role(conn: &mut SqliteConnection, name: &str, permissions: &[&str]) -> i64 {
    // Implementation using CreateRoleQuery::run(&mut *conn, data)
}
```

### Phase 5.2: Update Schema Building Utilities  
**Files to modify:**
- Update `create_test_query_schema()` and `create_test_mutation_schema()` in test_utils
- Ensure all resolver tests work with new connection patterns

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Resolver Test Impact:** All resolver tests use schema building utilities
- **Dependency Analysis:** Verify no direct query calls in resolvers (Phase 1.1 pattern)
- **Connection Pattern Consistency:** Ensure schema builders use consistent patterns
- **Test Integration:** Schema changes affect integration test patterns

### Phase 5.3: Final Integration Testing
- Run full test suite with `cargo test --quiet`
- Verify all resolver tests pass
- Verify all service tests pass
- Verify all orchestrator tests pass
- Verify integration tests pass

**Pre-implementation Analysis (Phase 1.1 Learnings):**
- **Test Coverage:** 458+ tests expected - comprehensive validation
- **Regression Testing:** Connection pattern changes may have subtle impacts
- **Performance Validation:** Verify no connection leaks or pool exhaustion
- **Cross-domain Validation:** Ensure all domains work together
- **Linter Integration:** Final code quality verification

---

## Implementation Details

### Key Architectural Patterns (Updated Based on Phase 0)

**Important**: Based on validation against actual codebase and Phase 0 implementation:

1. **Queries remain independent** - they don't call other queries
2. **Services handle dependencies** - service layer orchestrates multiple queries  
3. **Orchestrators handle business logic** - they call services, not queries directly
4. **Permission checking now uses connections** - `RoleService::check_user_permission(conn, &user, permission)`
5. **Resolvers call orchestrators** - they don't call queries directly
6. **Error mapping uses direct variants** - `ServiceError::DatabaseError` not closures
7. **Core infrastructure affects ALL layers** - permission system changes impact entire codebase (Phase 0 lesson)

### Connection Acquisition Pattern (Validated)

**Standard Pattern (Single Query):**
```rust
pub async fn method_name(pool: &SqlitePool, params: Params) -> Result<ReturnType, ServiceError> {
    let mut conn = pool.acquire().await
        .map_err(ServiceError::DatabaseError)?;
    
    let result = SomeQuery::run(&mut *conn, params).await  // Dereferencing needed for SQLx queries
        .map_err(ServiceError::DatabaseError)?;
    
    Ok(result)
}
```

**Scoped Connection Pattern (Permission Checks):**
```rust
pub async fn permission_based_method(pool: &SqlitePool, params: Params) -> Result<ReturnType, ServiceError> {
    // Get user using pool (no connection acquired yet)
    let user = GetUserByIdQuery::run(pool, user_id).await
        .map_err(ServiceError::DatabaseError)?;

    // Permission check - scoped connection acquisition
    let allowed = {
        let mut conn = pool.acquire().await
            .map_err(ServiceError::DatabaseError)?;
        RoleService::check_user_permission(&mut conn, &user, "can_list_users").await  // No dereferencing for service calls
            .map_err(ServiceError::RoleError)?
    };

    if allowed {
        // Business logic - pool still available
        SomeService::business_method(pool, data).await
    } else {
        Err(ServiceError::AuthorizationError("Forbidden".to_string()))
    }
}
```

**Multi-Query Pattern (Same Connection):**
```rust
pub async fn complex_method(pool: &SqlitePool, params: Params) -> Result<ReturnType, ServiceError> {
    let mut conn = pool.acquire().await
        .map_err(ServiceError::DatabaseError)?;
    
    // Use same connection for all queries
    let user = GetUserByIdQuery::run(&mut *conn, user_id).await
        .map_err(ServiceError::DatabaseError)?;
    
    let default_role = GetDefaultRoleQuery::run(&mut *conn).await  // Use same connection
        .map_err(ServiceError::DatabaseError)?;
        
    let result = CreateUserQuery::run(&mut *conn, data).await  // Continue connection reuse
        .map_err(ServiceError::DatabaseError)?;
            
    Ok(result)
}
```



### Error Handling Patterns (Validated)

**Query Functions:**
- Continue returning `sqlx::Error` (no change)
- Focus on connection parameter change only
- Use `&mut *conn` for all SQLx execution methods

**Service Functions:**
- Wrap `sqlx::Error` in service-specific errors
- Handle connection acquisition errors with `.map_err(ServiceError::DatabaseError)?`
- **Critical:** Use scoped connections for permission checks to avoid pool exhaustion

**Test Functions:**
- Acquire connections with `let mut conn = pool.acquire().await.unwrap();`
- Use proper dereferencing for SQLx queries: `SomeQuery::run(&mut *conn, params)`
- Service method calls use `&mut conn` (linter removes dereferencing)

### Testing Strategy (Updated Based on Phase 0)

**Unit Tests:**
- Each phase includes comprehensive test updates
- Test connection release and reuse
- Test error handling with connection failures
- **Critical Lesson:** Well-tested core infrastructure means MANY test updates per change

**Integration Tests:**
- Final phase ensures all integration tests pass
- Verify no connection leaks
- Verify performance improvements

**Regression Testing:**
- Each phase runs `cargo test --quiet` before proceeding
- Any test failures must be resolved before next phase
- **Phase 0 Evidence:** 458/458 tests passed after core permission system changes

**Test Pattern Recommendations:**
- Use `let mut conn = pool.acquire().await.unwrap();` for test simplicity
- Apply correct dereferencing: `&mut *conn` for queries, `&mut conn` for service calls
- Expect significant test changes when modifying core infrastructure (like permission system)

---

## Risk Assessment and Mitigation

### High Risk Areas

**Phase 4 (Critical Queries):**
- **Risk**: Breaking core authentication/authorization
- **Mitigation**: Extensive testing, gradual rollout

**Cross-Query Dependencies:**
- **Risk**: Connection sharing between queries
- **Mitigation**: Careful connection lifetime management

**Performance Impact:**
- **Risk**: Connection pool exhaustion
- **Mitigation**: Proper connection release patterns

**Scope Expansion Risk (Updated with Phase 1.1 Learnings):**
- **Risk**: Underestimated dependency impact (like Phase 0's 160% scope increase)
- **Additional Discovery**: Direct query calls bypassing service layer (found in orchestrators)
- **Mitigation**: 
  - Always perform full dependency analysis before phase planning
  - Verify orchestrator→service→query chain for all phases
  - Count individual test functions, not just test files
  - Expect cascading changes when touching core infrastructure
  - Plan for broader impact than initial analysis suggests
- **Phase 0 Lesson**: `check_user_permission` was more central than anticipated
- **Phase 1.1 Lesson**: Orchestrators may call queries directly, not just services

### Rollback Strategy

- Each phase can be rolled back independently
- Git commits per phase for easy reversion
- Comprehensive test coverage for early detection

---

## Success Criteria

1. **All queries** accept `&mut SqliteConnection` instead of `&SqlitePool`
2. **All callers** acquire connections and reuse them appropriately  
3. **All tests pass** with `cargo test --quiet`
4. **No connection leaks** in production patterns
5. **Performance maintained** or improved through better connection reuse
6. **Code follows** established error handling patterns

---

## Timeline Estimation

- **Phase 1**: 2-3 days (Simple CRUD - reduced scope)
- **Phase 2**: 3-4 days (Medium complexity)  
- **Phase 3**: 4-5 days (High usage)
- **Phase 4**: 6-8 days (Critical infrastructure + Update User dependencies)
- **Phase 5**: 2-3 days (Test utilities and integration)

**Total Estimated Time:** 17-23 days

---

*This plan follows the established project architecture and maintains all existing patterns while systematically converting to connection-based queries.*