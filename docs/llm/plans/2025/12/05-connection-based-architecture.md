# Connection-Based Database Architecture Plan

## Date Created
2025-12-05

## Problem Statement
Current architecture allows services and queries to directly access the connection pool, leading to potential concurrency issues where multiple operations in the same logical workflow execute on different connections, causing foreign key constraint failures and race conditions.

## Solution Overview
Implement a connection-based architecture where:
- **GraphQL Resolvers**: Accept pool from context (unchanged)
- **Orchestrators**: Accept pool, extract ONE connection, pass to services
- **Services/Queries**: Accept single connection instead of pool
- **Result**: All operations in a workflow use the same database connection

## Architecture Diagram

```
GraphQL Resolver (holds pool)
    ↓
Orchestrator (accepts pool, extracts connection)
    ↓ let mut conn = pool.acquire().await?
Service/Query (accepts &mut conn)
    ↓
Database Operation (uses single connection)
```

## Why Use Connection Pools with SQLite?

### SQLite Concurrency Model

**Traditional SQLite (without WAL)**:
- Single writer, multiple readers
- Writer locks entire database file
- Readers blocked during writes

**SQLite with WAL (Write-Ahead Logging)** - Used in This Project:
```rust
"PRAGMA journal_mode = WAL;"  // Line 54 in database/mod.rs
```
- Concurrent readers while writer is active
- Single writer at a time
- Better performance: readers don't block writers

### Connection Pool Benefits for SQLite

#### 1. Connection Setup Overhead
Each connection requires:
- Opening file handle
- Configuring 8+ PRAGMA settings (lines 51-74 in database/mod.rs)
- Setting up memory mapping (1MB per connection)
- Configuring cache settings (2MB per connection)

#### 2. Concurrent Request Handling
```rust
// Web server scenario:
Request 1 → Acquire Connection A → Process → Return A
Request 2 → Acquire Connection B → Process → Return B  
Request 3 → Wait for available connection → Acquire A → Process → Return A
```

#### 3. Connection Reuse Benefits
- **Warm Cache**: Connections retain prepared statements and cache
- **Reduced Latency**: No setup cost for subsequent requests
- **Resource Efficiency**: Limited number of file handles/memory allocations

### The Real Problem: Connection Consistency

The issue isn't pool efficiency - it's **connection consistency**:

```rust
// Current Problem:
async fn create_user_and_site(pool: &SqlitePool) {
    // Operation 1: Gets Connection A
    CreateUserQuery::run(pool, user_data).await?;
    
    // Operation 2: Gets Connection B  
    CreateSiteQuery::run(pool, site_data).await?; // ← Foreign key might fail!
}

// Solution:
async fn create_user_and_site(pool: &SqlitePool) {
    let mut conn = pool.acquire().await?;
    
    // Operation 1: Uses Connection A
    CreateUserQuery::run(&mut conn, user_data).await?;
    
    // Operation 2: Uses same Connection A
    CreateSiteQuery::run(&mut conn, site_data).await?; // ← Guaranteed to work!
}
```

### Pool Size Considerations for SQLite

```rust
// Typical SQLite pool sizes:
pool_size = 1  // Single-threaded, simple apps
pool_size = 5  // Small web apps (this project's default)
pool_size = 10 // Medium traffic web apps
pool_size = 20+ // High concurrency (rare for SQLite)
```

### When Pools Don't Make Sense for SQLite

- Single-threaded CLI tools
- Simple scripts  
- Low-traffic applications
- Embedded applications with single user

### When Pools Are Essential

- **Web servers** with concurrent requests
- **Applications with WAL mode** (like this project)
- **High-frequency operations** where connection setup cost matters
- **Complex transactions** requiring connection consistency

### Connection Lifecycle

```rust
pub async fn remove_site_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    site_id: i64,
) -> Result<(), SiteError> {
    // 1. Connection acquired from pool
    let mut conn = pool.acquire().await?;
    
    // 2. All operations use same connection
    check_permissions(&mut conn, &session_context).await?;
    SiteService::delete_site(&mut conn, site_id).await?;
    
    // 3. Function ends -> conn goes out of scope
    //    Connection automatically returned to pool
    Ok(())
}
```

**Benefits of Automatic Management**:
- No manual `release()` needed
- Exception safe: connection returned even on panics/errors
- RAII: Resource Acquisition Is Initialization pattern
- Pool efficiency: connection available for next request immediately

## Implementation Plan

### Phase 1: Proof of Concept with Isolated Database Path

**Selected Path**: `remove_site.rs` → `SiteOrchestrator::remove_site_with_permission_check` → `SiteService::delete_site` → `DeleteSiteQuery`

This path is ideal because:
- **Complete Isolation**: Each component is only used by its direct parent in production code
- **Database Operations**: Performs actual DELETE operations, testing connection-based architecture
- **Simple Implementation**: Straightforward CRUD operation with clear success criteria
- **Zero Risk**: No other functionality depends on these components

#### 1.1 Update DeleteSiteQuery Method Signature
**File**: `src/queries/sites/delete_site.rs`

**Current Pattern**:
```rust
impl DeleteSiteQuery {
    pub async fn run(pool: &SqlitePool, site_id: i64) -> Result<DeleteSiteResult, DatabaseError>
}
```

**New Pattern**:
```rust
impl DeleteSiteQuery {
    pub async fn run(conn: &mut SqliteConnection, site_id: i64) -> Result<DeleteSiteResult, DatabaseError>
}
```

#### 1.2 Update SiteService::delete_site Method
**File**: `src/services/site_service.rs`

**Current Pattern**:
```rust
impl SiteService {
    pub async fn delete_site(pool: &SqlitePool, site_id: i64) -> Result<(), SiteError>
}
```

**New Pattern**:
```rust
impl SiteService {
    pub async fn delete_site(conn: &mut SqliteConnection, site_id: i64) -> Result<(), SiteError>
}
```

#### 1.3 Update SiteOrchestrator::remove_site_with_permission_check
**File**: `src/orchestrators/site_orchestrator.rs`

**Current Pattern**:
```rust
impl SiteOrchestrator {
    pub async fn remove_site_with_permission_check(
        pool: &SqlitePool,
        session_context: SessionContext,
        site_id: i64,
    ) -> Result<(), SiteError>
}
```

**New Pattern**:
```rust
impl SiteOrchestrator {
    pub async fn remove_site_with_permission_check(
        pool: &SqlitePool,
        session_context: SessionContext,
        site_id: i64,
    ) -> Result<(), SiteError> {
        let mut conn = pool.acquire().await?;
        
        // Authentication and authorization checks using same connection
        // ... existing auth logic ...
        
        // Business logic using same connection
        SiteService::delete_site(&mut conn, site_id).await?;
        
        Ok(())
    }
}
```

#### 1.4 Update RemoveSiteResolver
**File**: `src/graphql/resolvers/remove_site.rs`

**Current Pattern**:
```rust
impl RemoveSiteResolver {
    async fn remove_site(&self, ctx: &Context<'_>, site_id: i64) -> Result<RemoveSiteResponse> {
        let pool = ctx.data::<SqlitePool>()?;
        let session_context = get_session_context(ctx)?;
        let result = SiteOrchestrator::remove_site_with_permission_check(pool, session_context, site_id).await?;
        // ...
    }
}
```

**New Pattern** (no change needed - resolver continues to pass pool):
```rust
impl RemoveSiteResolver {
    async fn remove_site(&self, ctx: &Context<'_>, site_id: i64) -> Result<RemoveSiteResponse> {
        let pool = ctx.data::<SqlitePool>()?;
        let session_context = get_session_context(ctx)?;
        let result = SiteOrchestrator::remove_site_with_permission_check(pool, session_context, site_id).await?;
        // ...
    }
}
```

#### 1.5 Update Tests
**Files**: Tests for delete_site, remove_site functionality

**Current Pattern**:
```rust
#[tokio::test]
async fn test_delete_site() {
    let (pool, _temp_file) = create_test_database().await;
    let site_id = create_test_site(&pool, "test.com").await;
    let result = SiteService::delete_site(&pool, site_id).await;
    assert!(result.is_ok());
}
```

**New Pattern**:
```rust
#[tokio::test]
async fn test_delete_site() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();
    let site_id = create_test_site(&mut conn, "test.com").await;
    let result = SiteService::delete_site(&mut conn, site_id).await;
    assert!(result.is_ok());
}
```

## Future Phases: Implementation Roadmap

### Phase 2: Fully Isolated Paths (Low Risk)
**Priority**: HIGH - Simple utilities with no shared dependencies

1. **getServerTimestamp**
   - Path: `GetServerTimestampResolver` → `ServerService::get_server_timestamp`
   - Isolation: Fully Isolated
   - Description: Simple utility function with no database dependencies
   - Files: `src/services/server_service.rs`, `src/graphql/resolvers/get_server_timestamp.rs`

2. **authLogout**
   - Path: `AuthLogoutResolver` → (no service/orchestrator)
   - Isolation: Fully Isolated
   - Description: Only handles cookie clearing, no business logic
   - Files: `src/graphql/resolvers/auth_logout.rs`

### Phase 3: Partially Isolated Paths (Core Features)
**Priority**: HIGH - Essential functionality with minimal shared components

3. **sites** (list all sites)
   - Path: `SitesResolver` → `SiteService::get_all_sites` → `GetAllSitesQuery`
   - Isolation: Partially Isolated
   - Shared Components: `SiteService` (used by other site operations)
   - Files: `src/graphql/resolvers/sites.rs`, `src/services/site_service.rs`, `src/queries/sites/get_all_sites.rs`

4. **authLogin**
   - Path: `AuthLoginResolver` → `AuthService::login` → `SessionService::create_session` + `GetUserByNameQuery`
   - Isolation: Partially Isolated
   - Shared Components: `AuthService`, `SessionService`, `GetUserByNameQuery`
   - Files: `src/graphql/resolvers/auth_login.rs`, `src/services/auth_service.rs`, `src/services/session_service.rs`, `src/queries/users/get_user_by_name.rs`

5. **authRegister**
   - Path: `AuthRegisterResolver` → `AuthService::register` → `UserService::create_user` + `GetDefaultUserRoleQuery`
   - Isolation: Partially Isolated
   - Shared Components: `AuthService`, `UserService`, user-related queries
   - Files: `src/graphql/resolvers/auth_register.rs`, `src/services/auth_service.rs`, `src/services/user_service.rs`, `src/queries/users/create_user.rs`, `src/queries/user_roles/get_default_user_role.rs`

6. **authMe**
   - Path: `AuthMeResolver` → `AuthService::get_current_user` → `SessionService::validate_session` + `GetUserByIdQuery`
   - Isolation: Partially Isolated
   - Shared Components: `AuthService`, `SessionService`, `GetUserByIdQuery`
   - Files: `src/graphql/resolvers/auth_me.rs`, `src/services/auth_service.rs`, `src/services/session_service.rs`, `src/queries/users/get_user_by_id.rs`

### Phase 4: Highly Coupled Paths (Advanced Features)
**Priority**: MEDIUM - Complex operations with extensive shared components

7. **authChangePassword**
   - Path: `AuthChangePasswordResolver` → `AuthOrchestrator::change_authenticated_user_password` → `UserService::update_password` + `GetUserByIdQuery`
   - Isolation: Highly Coupled
   - Shared Components: `UserService` (shared across many user operations), user queries
   - Files: `src/graphql/resolvers/auth_change_password.rs`, `src/orchestrators/auth_orchestrator.rs`, `src/services/user_service.rs`, `src/queries/users/update_user_password.rs`

8. **site** (get site details)
   - Path: `SiteResolver` → `SiteOrchestrator::get_site_details_with_permission_check` → `GetSiteByIdQuery` + `UserRoleService::check_user_permission` + `GetUserByIdQuery`
   - Isolation: Highly Coupled
   - Shared Components: `SiteOrchestrator`, `UserRoleService`, multiple queries
   - Files: `src/graphql/resolvers/site.rs`, `src/orchestrators/site_orchestrator.rs`, `src/services/user_role_service.rs`, `src/queries/sites/get_site_by_id.rs`

9. **addSite**
   - Path: `AddSiteResolver` → `SiteOrchestrator::create_site_with_permission_check` → `SiteService::create_site` + `UserRoleService::check_user_permission` + `GetUserByIdQuery`
   - Isolation: Highly Coupled
   - Shared Components: `SiteOrchestrator`, `SiteService`, `UserRoleService`
   - Files: `src/graphql/resolvers/add_site.rs`, `src/orchestrators/site_orchestrator.rs`, `src/services/site_service.rs`, `src/queries/sites/create_site.rs`

10. **updateSite**
    - Path: `UpdateSiteResolver` → `SiteOrchestrator::update_site_with_permission_check` → `SiteService::update_site` + `UserRoleService::check_user_permission` + `GetUserByIdQuery`
    - Isolation: Highly Coupled
    - Shared Components: `SiteOrchestrator`, `SiteService`, `UserRoleService`
    - Files: `src/graphql/resolvers/update_site.rs`, `src/orchestrators/site_orchestrator.rs`, `src/services/site_service.rs`, `src/queries/sites/update_site.rs`

11. **removeSite**
    - Path: `RemoveSiteResolver` → `SiteOrchestrator::remove_site_with_permission_check` → `SiteService::delete_site` + `UserRoleService::check_user_permission` + `GetUserByIdQuery`
    - Isolation: Highly Coupled
    - Shared Components: `SiteOrchestrator`, `SiteService`, `UserRoleService`
    - Files: `src/graphql/resolvers/remove_site.rs`, `src/orchestrators/site_orchestrator.rs`, `src/services/site_service.rs`, `src/queries/sites/delete_site.rs`

### Phase 5: User Management (Highest Complexity)
**Priority**: LOW - Most complex operations with highest risk

12. **user** (get user details)
    - Path: `UserResolver` → `UserOrchestrator::get_user_details_with_permission_check` → `GetUserByIdWithRoleQuery` + `UserRoleService::check_user_permission` + `GetUserByIdQuery`
    - Isolation: Highly Coupled
    - Shared Components: `UserOrchestrator`, `UserRoleService`, multiple user queries
    - Files: `src/graphql/resolvers/user.rs`, `src/orchestrators/user_orchestrator.rs`, `src/services/user_role_service.rs`, `src/queries/users/get_user_by_id_with_role.rs`

13. **users** (list users)
    - Path: `UsersResolver` → `UserOrchestrator::list_users_with_permission_check` → `GetAllUsersWithRolesQuery` + `UserRoleService::check_user_permission` + `GetUserByIdQuery`
    - Isolation: Highly Coupled
    - Shared Components: `UserOrchestrator`, `UserRoleService`, user queries
    - Files: `src/graphql/resolvers/users.rs`, `src/orchestrators/user_orchestrator.rs`, `src/services/user_role_service.rs`, `src/queries/users/get_all_users_with_roles.rs`

14. **deleteUser**
    - Path: `DeleteUserResolver` → `UserOrchestrator::delete_user_with_permission_check` → `UserService::delete_user` + `UserRoleService::check_user_permission` + `GetUserByIdQuery`
    - Isolation: Highly Coupled
    - Shared Components: `UserOrchestrator`, `UserService`, `UserRoleService`
    - Files: `src/graphql/resolvers/delete_user.rs`, `src/orchestrators/user_orchestrator.rs`, `src/services/user_service.rs`, `src/queries/users/delete_user_by_id.rs`

## Implementation Strategy

### Key Dependencies to Implement First
1. **Database Layer**: All query objects (foundation)
2. **Core Services**: `PasswordService`, `SessionService`, `ServerService`
3. **User Management**: `UserService`, `UserRoleService`
4. **Authentication**: `AuthService`
5. **Site Management**: `SiteService`
6. **Orchestrators**: `AuthOrchestrator`, `SiteOrchestrator`, `UserOrchestrator`

### Progression Approach
- **Start Simple**: Utilities and isolated paths first
- **Build Foundation**: Core authentication and user management
- **Add Complexity**: Permission-based operations last
- **Test Thoroughly**: Each phase validated before proceeding

This roadmap allows for incremental implementation across multiple sessions, with clear priorities and isolation levels to minimize risk and maximize learning at each step.

## Implementation Details

### Connection Type
Use `sqlx::SqliteConnection` for all service/query parameters:
```rust
use sqlx::{SqlitePool, SqliteConnection};

pub async fn example_service(conn: &mut SqliteConnection) -> Result<(), Error> {
    // Database operations using conn
    sqlx::query("SELECT * FROM users")
        .fetch_all(conn)
        .await
}
```

### Error Handling
Maintain existing error types - only change parameter types:
```rust
// Before
pub async fn create_user(pool: &SqlitePool, ...) -> Result<User, UserError>

// After  
pub async fn create_user(conn: &mut SqliteConnection, ...) -> Result<User, UserError>
```

### Async/Await Patterns
All methods remain async - connection operations are still async:
```rust
pub async fn create_user(conn: &mut SqliteConnection, ...) -> Result<User, UserError> {
    // Connection operations are async even with single connection
    let result = sqlx::query("INSERT INTO users ...")
        .execute(conn)
        .await?;
    // ...
}
```

## Benefits

### 1. Eliminates Concurrency Issues
- All operations in workflow use same connection
- Foreign key constraints work correctly
- No race conditions between related operations

### 2. Simple Implementation
- Clear architectural boundaries
- Minimal code changes in resolvers
- Straightforward parameter type changes

### 3. Maintains Performance
- Connection pooling still works at orchestrator level
- No connection overhead for single operations
- Pool can still handle concurrent requests

### 4. Backward Compatible
- GraphQL resolver interface unchanged
- Public API remains the same
- Only internal implementation changes

## Migration Strategy

### Step 1: Proof of Concept (Phase 1)
1. Update `ServerService::get_server_timestamp()` to accept `&mut SqliteConnection`
2. Update `GetServerTimestampResolver` to extract connection and pass to service
3. Update related tests to use connection pattern
4. Verify tests pass and functionality unchanged

### Step 2: Validate Architecture
1. Confirm connection-based pattern works for isolated path
2. Measure any performance impact
3. Refine implementation approach if needed

### Step 3: Expand to Additional Paths (Phase 2)
1. Identify next isolated paths (e.g., sites functionality)
2. Apply same pattern incrementally
3. Test each path independently

### Step 4: Full Migration (Phase 3)
1. Apply connection-based architecture to remaining paths
2. Update orchestrators to extract connections
3. Update all remaining services and queries
4. Update all tests

### Step 5: Clean Up
1. Remove any remaining pool usage in services/queries
2. Update documentation
3. Run linting and formatting

## Files Requiring Changes

### Phase 1: Proof of Concept (Immediate)
- `src/queries/sites/delete_site.rs` (1 method signature)
- `src/services/site_service.rs` (1 method signature)
- `src/orchestrators/site_orchestrator.rs` (1 method signature)
- `src/graphql/resolvers/remove_site.rs` (no changes needed)
- Related test files

### Future Phases: Complete Migration List

#### Core Files (High Priority)
- `src/services/user_service.rs` (~20 method signatures)
- `src/services/auth_service.rs` (~5 method signatures)
- `src/services/site_service.rs` (~8 method signatures)
- `src/services/session_service.rs` (~5 method signatures)
- `src/services/user_role_service.rs` (~3 method signatures)
- `src/orchestrators/user_orchestrator.rs` (~10 method signatures)
- `src/orchestrators/auth_orchestrator.rs` (~3 method signatures)
- `src/orchestrators/site_orchestrator.rs` (~5 method signatures)

#### Query Files (Medium Priority)
- All files in `src/queries/users/` (~9 files)
- All files in `src/queries/sites/` (~4 files)
- All files in `src/queries/user_roles/` (~4 files)

#### GraphQL Resolver Files (Medium Priority)
- All files in `src/graphql/resolvers/` (~15 files)

#### Test Files (Low Priority)
- All test files in `src/*/tests/` modules
- Integration tests in `tests/` directory

## Risk Assessment

### Low Risk
- **No API Changes**: External interface unchanged
- **Backward Compatible**: Existing functionality preserved
- **Incremental**: Can be implemented step by step

### Medium Risk
- **Large Surface Area**: Many files need updates
- **Test Coverage**: Need to ensure all paths tested
- **Connection Management**: Need proper error handling for `pool.acquire()`

### Mitigation
- Implement incrementally with testing at each step
- Maintain existing error handling patterns
- Use Rust's type system to catch issues at compile time

## Success Criteria

### Phase 1: Proof of Concept
- [ ] `RemoveSiteResolver` successfully deletes sites using connection-based architecture
- [ ] All related tests pass (query, service, orchestrator, resolver)
- [ ] No performance regression
- [ ] Code compiles without warnings
- [ ] Database operations use single connection throughout the call chain

### Full Migration
- [ ] All existing tests pass
- [ ] No foreign key constraint errors
- [ ] GraphQL operations work correctly
- [ ] Authentication/authorization flows work

### Performance
- [ ] No regression in request throughput
- [ ] Connection pool utilization remains efficient
- [ ] Database query performance maintained

### Code Quality
- [ ] All code compiles without warnings
- [ ] Linter passes without issues
- [ ] Documentation updated where needed

## Timeline

### Day 1: Proof of Concept
- Update `ServerService` and `GetServerTimestampResolver`
- Update related tests
- Verify functionality works

### Day 2: Validation
- Run comprehensive tests
- Measure performance impact
- Document lessons learned

### Week 1: Additional Paths
- Apply pattern to 2-3 additional isolated paths
- Test each independently
- Refine approach based on experience

### Week 2-3: Full Migration
- Apply connection-based architecture to remaining paths
- Update orchestrators, services, and queries
- Update all tests

### Week 4: Testing & Polish
- Full test suite validation
- Performance testing
- Documentation updates

## Conclusion

This connection-based architecture provides a simple, elegant solution to the concurrency issues while maintaining the existing API structure. By ensuring all operations in a single workflow use the same database connection, we eliminate foreign key constraint failures and race conditions without complex transaction management or pool size limitations.

The approach is incremental, testable, and maintains backward compatibility while providing a clear architectural boundary between connection management (orchestrators) and business logic (services/queries).