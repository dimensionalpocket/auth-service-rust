# Plan: Continue Query Pool-to-Connection Migration

**Created:** 2025-12-27

## Overview

This plan continues the work of migrating query objects from using `SqlitePool` to `SqliteConnection`. Some query objects have already been updated to use connections. This plan identifies the remaining queries that still use pools and prioritizes their migration.

## Current State Analysis

### Queries Already Using Connections (No Changes Needed)

These queries have already been migrated to use connections:
- `DeleteRoleQuery` (roles/delete_role.rs) - uses `SqliteConnection`
- `GetAllRolesQuery` (roles/get_all_roles.rs) - uses `SqliteConnection`
- `GetRoleByIdQuery` (roles/get_role_by_id.rs) - uses `SqliteConnection`
- `GetRoleByNameQuery` (roles/get_role_by_name.rs) - uses `SqliteConnection`
- `SetDefaultRoleQuery` (roles/set_default_role.rs) - uses `SqliteConnection`
- `UpdateRoleQuery` (roles/update_role.rs) - uses `SqliteConnection`
- `DeleteSiteQuery` (sites/delete_site.rs) - uses `SqliteConnection`
- `GetAllSitesQuery` (sites/get_all_sites.rs) - uses `SqliteConnection`
- `GetSiteByIdQuery` (sites/get_site_by_id.rs) - uses `SqliteConnection`
- `UpdateSiteQuery` (sites/update_site.rs) - uses `SqliteConnection`
- `DeleteUserByIdQuery` (users/delete_user_by_id.rs) - uses `SqliteConnection`
- `GetAllUsersWithRolesQuery` (users/get_all_users_with_roles.rs) - uses `SqliteConnection`
- `UpdateUserPasswordQuery` (users/update_user_password.rs) - uses `SqliteConnection`

### Queries Still Using Pools

The following query objects still use `SqlitePool` and need to be migrated:

#### 1. GetDefaultRoleQuery (roles/get_default_role.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- RoleService: 2 calls (lines 1086, 1103)
- CreateUserQuery: 2 calls (lines 25, 170)
- Self tests: 3 calls
- TestUtils: 3 calls (lines 103, 141, 310)

**Priority:** HIGH - Called by CreateUserQuery which is itself a pool-based query. This creates a circular dependency.

**Complexity:** LOW - Simple SELECT query

#### 2. GetUserByUuidQuery (users/get_user_by_uuid.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- DatabaseIntegrationTests: 1 call (line 43)
- Self tests: 2 calls
- TestUtils: 1 call (line 596)

**Priority:** MEDIUM - Only called by tests, not used in production code paths

**Complexity:** LOW - Simple SELECT query

#### 3. GetUserByNameQuery (users/get_user_by_name.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- UserService: 3 calls (lines 93, 132, 322)
- Self tests: 5 calls

**Priority:** HIGH - Called by UserService in production code paths

**Complexity:** LOW - Simple SELECT query

#### 4. GetUserByNameWithRoleQuery (users/get_user_by_name_with_role.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- AuthService: 2 calls (lines 75, 132)
- Self tests: 6 calls

**Priority:** HIGH - Called by AuthService for login operations

**Complexity:** LOW - Simple SELECT with JOIN

#### 5. GetUserByIdWithRoleQuery (users/get_user_by_id_with_role.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- UserOrchestrator: 1 call (line 106)
- AuthService: 1 call (line 178)
- Self tests: 5 calls

**Priority:** HIGH - Called by AuthService and UserOrchestrator in production

**Complexity:** LOW - Simple SELECT with JOIN

#### 6. GetUserByIdQuery (users/get_user_by_id.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- UserService: 4 calls (lines 149, 218, 311, 511, 523, 742, 750)
- UserOrchestrator: 6 calls (lines 39, 90, 141, 206, 583, 673)
- UpdateUserQuery: 1 call (line 66)
- SiteOrchestrator: 4 calls (lines 37, 69, 102, 134)
- RolePermissions resolver: 1 call (line 31)
- RoleOrchestrator: 7 calls (lines 27, 67, 107, 142, 181, 216, 1376)
- Self tests: 2 calls
- AuthOrchestrator: 1 call (line 27)

**Priority:** HIGH - Very widely used across multiple services, orchestrators, and resolvers

**Complexity:** LOW - Simple SELECT query

#### 7. UpdateUserQuery (users/update_user.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- UserService: 1 call (line 336)
- Self tests: 10 calls

**Priority:** MEDIUM - Called by 1 service, but complex with RETURNING clause

**Complexity:** HIGH - Uses dynamic SQL building with RETURNING clause (SQLite-specific)

#### 8. CreateUserQuery (users/create_user.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- DatabaseIntegrationTests: 1 call (line 87)
- RoleService: 1 call (line 1042)
- DeleteRoleQuery test: 1 call (line 114)
- UserService: 1 call (line 113) + 15 calls in tests
- UpdateUserQuery tests: 10 calls
- Self tests: 5 calls
- TestUtils: 2 calls (lines 127, 165)
- User resolver test: 1 call (line 124)

**Priority:** HIGH - Called by RoleService and UserService in production

**Complexity:** MEDIUM - Calls GetDefaultRoleQuery (another pool-based query)

#### 9. CreateRoleQuery (roles/create_role.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- RoleService: 1 call (line 133)
- DeleteRoleQuery tests: 2 calls
- TestUtils: 1 call (line 186)
- Self tests: 6 calls

**Priority:** MEDIUM - Called by RoleService in production

**Complexity:** LOW - Simple INSERT query

#### 10. CreateSiteQuery (sites/create_site.rs)

**Current State:** Uses `&SqlitePool`

**Callers:**
- SiteService: 1 call (line 71)
- Multiple test files (26 total calls across test files and self tests)

**Priority:** MEDIUM - Called by SiteService in production

**Complexity:** LOW - Simple INSERT query with RETURNING

## Migration Phases

### Connection Acquisition Strategy

### Important Rule of Thumb

**Never hold a connection when the caller is passing a pool to another function or object.** With a pool size of 1, once a connection is acquired from the pool, the pool will be empty and cannot be used by any other caller until that connection is released back to the pool.

### Obscure Pattern in Resolver Tests: schema.execute() Uses Pool

In resolver tests (e.g., `test_remove_site_success` in `src/graphql/resolvers/remove_site.rs`), there's a subtle pattern where the pool is used indirectly even when not explicitly referenced:

```rust
// Example pattern:
let site = {
    let mut conn = pool.acquire().await?;
    CreateSiteQuery::run(&mut conn, site_data).await?
};

// No explicit pool reference here, but schema.execute(query) uses the pool internally!
let result = schema.execute(query).await?;
```

**Important:** Even if there's no explicit pool variable in the method, `schema.execute(query)` requires an available pool connection. If a connection is held from a previous acquisition (e.g., outside of a scoped block), the pool will be empty and the schema execution will fail.

**Correct pattern for tests:**
```rust
// Always use scoped blocks for test setup
let site_id = {
    let mut conn = pool.acquire().await?;
    let site = CreateSiteQuery::run(&mut conn, site_data).await?;
    site.id
}; // Connection released here

// Now pool is available for schema.execute()
let result = schema.execute(query).await?;
```

### Analysis of Current Patterns

Based on analysis of service and orchestrator code, three main patterns emerge for how callers interact with queries:

**Pattern 1: Caller already acquires connection and passes it**
- **RoleService.get_all_roles** (line 60): Acquires connection, passes to GetAllRolesQuery ✓
- **RoleService.get_role_by_id** (line 69): Acquires connection, passes to GetRoleByIdQuery ✓
- **RoleService.get_role_by_name** (line 78): Acquires connection, passes to GetRoleByNameQuery ✓
- **RoleService.set_default_role** (line 235): Acquires connection, passes to SetDefaultRoleQuery ✓
- **RoleService.update_role** (line 210): Acquires connection, passes to UpdateRoleQuery ✓
- **RoleService.delete_role** (line 166): Uses pool for sqlx::query, then acquires connection for DeleteRoleQuery ✓
- **UserService.update_password** (line 241): Acquires connection, passes to UpdateUserPasswordQuery ✓
- **UserService.delete_user** (line 260): Acquires connection, passes to DeleteUserByIdQuery ✓

**Pattern 2: Caller passes pool directly to query, no connection held**
- **UserService.create_user** (line 113): Passes pool to CreateUserQuery, then returns
- **UserService.get_user_by_name** (line 132): Passes pool to GetUserByNameQuery, then returns
- **UserService.get_user_by_id** (line 149): Passes pool to GetUserByIdQuery, then returns
- **UserService.update_user** (line 336): Passes pool to UpdateUserQuery, then returns
- **AuthService.login** (line 75): Passes pool to GetUserByNameWithRoleQuery, then returns
- **AuthService.register** (line 132): Calls UserService.create_user (pool), then GetUserByNameWithRoleQuery (pool)
- **AuthService.get_current_user** (line 178): Passes pool to GetUserByIdWithRoleQuery, then returns
- **RoleService.create_role** (lines 118, 133): Uses pool for GetRoleByNameQuery, then passes pool to CreateRoleQuery
- **SiteService.create_site** (line 71): Passes pool to CreateSiteQuery, then returns

**Pattern 3: Caller mixes pool and connection usage**
- **UserService.update_password** (line 218): Passes pool to GetUserByIdQuery, then acquires connection for UpdateUserPasswordQuery
- **UserService.update_user** (line 311): Passes pool to GetUserByIdQuery, then passes pool to GetUserByNameQuery, then passes pool to UpdateUserQuery
- **UserService.update_user** (line 322): Passes pool to GetUserByNameQuery, then passes pool to UpdateUserQuery
- **AuthOrchestrator.change_authenticated_user_password** (line 27): Passes pool to GetUserByIdQuery, then calls UserService.update_password (pool)
- **UserOrchestrator.list_users_with_permission_check** (line 39): Passes pool to GetUserByIdQuery, then acquires connection for RoleService.check_user_permission, then acquires connection for GetAllUsersWithRolesQuery
- **UserOrchestrator.get_user_details_with_permission_check** (line 90): Passes pool to GetUserByIdQuery, then acquires connection for permission check, then passes pool to GetUserByIdWithRoleQuery
- **UserOrchestrator.delete_user_with_permission_check** (line 41): Passes pool to GetUserByIdQuery, then acquires connection for permission check, then acquires connection for DeleteUserByIdQuery
- **UserOrchestrator.update_user_with_permission_check** (lines ~133, ~200): Passes pool to GetUserByIdQuery, acquires connection for permission check, then acquires connection for update
- **RoleOrchestrator.get_all_roles_with_permission_check** (line 27): Passes pool to GetUserByIdQuery, acquires connection for permission checks, then calls RoleService.get_all_roles (pool)
- **RoleOrchestrator.get_role_by_id_with_permission_check** (line 67): Passes pool to GetUserByIdQuery, acquires connection for permission check, then calls RoleService.get_role_by_id (pool)
- **RoleOrchestrator.update_role_with_permission_check** (line 107): Passes pool to GetUserByIdQuery, acquires connection for permission check, then calls RoleService.update_role (pool)
- **RoleOrchestrator.create_role_with_permission_check** (line 142): Passes pool to GetUserByIdQuery, acquires connection for permission check, then calls RoleService.create_role (pool)
- **RoleOrchestrator.delete_role_with_permission_check** (line ~216): Passes pool to GetUserByIdQuery, acquires connection for permission check, then calls RoleService.delete_role (pool)
- **RoleOrchestrator.set_default_role_with_permission_check** (line ~181): Passes pool to GetUserByIdQuery, acquires connection for permission check, then calls RoleService.set_default_role (pool)
- **SiteOrchestrator.create_site_with_permission_check** (line 37): Passes pool to GetUserByIdQuery, acquires connection for permission check, then calls SiteService.create_site (pool)
- **SiteOrchestrator.remove_site_with_permission_check** (line 69): Passes pool to GetUserByIdQuery, acquires connection for permission check, then calls SiteService.delete_site (pool)
- **SiteOrchestrator.update_site_with_permission_check** (line 102): Passes pool to GetUserByIdQuery, acquires connection for permission check, then calls SiteService.update_site (pool)
- **SiteOrchestrator.get_site_details_with_permission_check** (line 134): Passes pool to GetUserByIdQuery, acquires connection for permission check, acquires connection for GetSiteByIdQuery
- **GraphQL resolvers**: Various patterns, mostly pass pool to multiple queries in sequence

### Caller Migration Strategy

Based on the above analysis, here's the strategy for migrating callers:

**Case 1: Caller currently not acquiring a connection, pool is used after connection is needed**

**Strategy:** Acquire connection in a block for the query, then release it before next query that needs pool.

**Example:** UserService.create_user (currently passes pool to CreateUserQuery)
```rust
// Current:
let user = CreateUserQuery::run(pool, create_data).await?;

// After Phase 8 (CreateUserQuery and GetDefaultRoleQuery both use connections):
{
    let mut conn = pool.acquire().await?;
    let user = CreateUserQuery::run(&mut conn, create_data).await?;
} // Connection released here
```

**Affected callers:**
- UserService.create_user
- UserService.get_user_by_name
- UserService.get_user_by_id
- UserService.update_user
- AuthService.login
- AuthService.register
- AuthService.get_current_user
- RoleService.create_role
- SiteService.create_site

**Complexity:** LOW - Simple pattern, just wrap query call in acquisition block.

**Case 2: Caller already acquires a connection**

**Strategy:** Pass that same connection to the updated query object.

**Example:** RoleService.get_all_roles (already acquires connection for GetAllRolesQuery)
```rust
// Current:
let mut conn = pool.acquire().await?;
GetAllRolesQuery::run(&mut conn).await?;

// After Phase 1-7 (GetUserByIdQuery and other simple queries use connections):
// No changes needed - already using connection pattern correctly!
```

**Affected callers:**
- RoleService.get_all_roles ✓ (no changes needed)
- RoleService.get_role_by_id ✓ (no changes needed)
- RoleService.get_role_by_name ✓ (no changes needed)
- RoleService.set_default_role ✓ (no changes needed)
- RoleService.update_role ✓ (no changes needed)
- RoleService.delete_role ✓ (no changes needed)
- UserService.update_password ✓ (no changes needed)
- UserService.delete_user ✓ (no changes needed)

**Complexity:** NONE - Already following correct pattern.

**Case 3: Caller receives pool as argument but does not use it (after refactoring)**

**Strategy:** After all queries called by the method are updated to use connections, acquire connection once at method start and pass to all queries.

**Example:** UserOrchestrator.list_users_with_permission_check
```rust
// Current:
let user = GetUserByIdQuery::run(pool, user_id).await?;
let allowed = {
    let mut conn = pool.acquire().await?;
    RoleService::check_user_permission(&mut conn, &user, "can_list_users").await?
};
let mut conn = pool.acquire().await?;
GetAllUsersWithRolesQuery::run(&mut conn).await?;

// After Phase 5 (GetUserByIdQuery uses connections) + Phase 2 (GetAllUsersWithRolesQuery already uses connections):
let mut conn = pool.acquire().await?;
let user = GetUserByIdQuery::run(&mut conn, user_id).await?;
let allowed = RoleService::check_user_permission(&mut conn, &user, "can_list_users").await?;
GetAllUsersWithRolesQuery::run(&mut conn).await?;
// All queries use same connection
```

**Affected callers requiring refactoring:**
- **UserOrchestrator.list_users_with_permission_check** - After Phase 5, can consolidate
- **UserOrchestrator.get_user_details_with_permission_check** - After Phase 5, can consolidate
- **UserOrchestrator.delete_user_with_permission_check** - After Phase 5, can consolidate
- **UserOrchestrator.update_user_with_permission_check** - After Phase 5, can consolidate
- **RoleOrchestrator.get_all_roles_with_permission_check** - After Phase 5, can consolidate
- **RoleOrchestrator.get_role_by_id_with_permission_check** - After Phase 5, can consolidate
- **RoleOrchestrator.update_role_with_permission_check** - After Phase 5, can consolidate
- **RoleOrchestrator.create_role_with_permission_check** - After Phase 8, can consolidate
- **RoleOrchestrator.delete_role_with_permission_check** - After Phase 5, can consolidate
- **RoleOrchestrator.set_default_role_with_permission_check** - After Phase 5, can consolidate
- **SiteOrchestrator.create_site_with_permission_check** - After Phase 5, can consolidate
- **SiteOrchestrator.remove_site_with_permission_check** - After Phase 5, can consolidate
- **SiteOrchestrator.update_site_with_permission_check** - After Phase 5, can consolidate
- **SiteOrchestrator.get_site_details_with_permission_check** - After Phase 5, can consolidate
- **GraphQL resolvers** - After phases complete, can consolidate multiple sequential query calls

**Complexity:** MEDIUM to HIGH - Requires careful orchestration of when to acquire and release connections, must ensure no pool usage while connection is held.

**Note on Partial Consolidation:** When a caller finishes all pool-based operations and has only connection-based queries remaining, acquire a single connection for all remaining queries instead of separate scoped blocks. This is an intermediate optimization that doesn't require waiting for all phases. Example: After GetUserByIdQuery runs (pool-based), if remaining operations are only permission checks and GetUserByIdWithRoleQuery (both connection-based), acquire one connection and reuse it.

**Case 4: Caller with pool → pool → pool pattern that crosses phases**

**Strategy:** Do NOT refactor during the query update phase. Mark for review after all relevant phases are complete.

**Example:** AuthOrchestrator.change_authenticated_user_password
```rust
// Current:
let user = GetUserByIdQuery::run(pool, user_id).await?;
UserService::update_password(pool, user_id, ...).await;

// After Phase 5 (GetUserByIdQuery uses connections) but before Phase 9 (UpdateUserQuery still uses pool):
// Would need: acquire conn → GetUserByIdQuery → release conn → acquire conn → UpdateUserQuery → release conn
// OR: Wait until Phase 9 completes, then refactor both together
```

**Affected callers requiring review:**
- **AuthOrchestrator.change_authenticated_user_password** (calls GetUserByIdQuery → UserService.update_password)
  - GetUserByIdQuery migrates in Phase 5
  - UpdateUserQuery migrates in Phase 9
  - **Recommendation:** Review and refactor after both phases complete
- **UserService.update_password** (calls GetUserByIdQuery → UpdateUserPasswordQuery)
  - GetUserByIdQuery migrates in Phase 5
  - UpdateUserPasswordQuery already uses connections (migrated)
  - **Recommendation:** Review after Phase 5 - can consolidate
- **UserService.update_user** (calls GetUserByIdQuery → GetUserByNameQuery → UpdateUserQuery)
  - GetUserByIdQuery migrates in Phase 5
  - GetUserByNameQuery migrates in Phase 2
  - UpdateUserQuery migrates in Phase 9
  - **Recommendation:** Review after Phase 9 - can consolidate all three
- **AuthService.register** (calls UserService.create_user → GetUserByNameWithRoleQuery → SessionService)
  - CreateUserQuery migrates in Phase 8
  - GetUserByNameWithRoleQuery migrates in Phase 3
  - **Recommendation:** Review after Phase 8 - can consolidate first two

**Complexity:** HIGH - Requires coordinating across multiple phases, deferring refactoring to avoid temporary mixed patterns.

## Callers Requiring Further Review

## Callers Requiring Further Review

The following callers cross phase boundaries and require careful review after relevant phases are complete:

### Review After Phase 5 (GetUserByIdQuery migrates):

1. **UserService.update_password** (user_service.rs:200-248)
   - Current pattern:
     ```rust
     let user = GetUserByIdQuery::run(pool, user_id).await?;
     // ... password verification ...
     let mut conn = pool.acquire().await?;
     let updated_user = UpdateUserPasswordQuery::run(&mut conn, user_id, update_data).await?;
     ```
   - Problem: GetUserByIdQuery uses pool, then connection is acquired for UpdateUserPasswordQuery
   - GetUserByIdQuery migrates in Phase 5
   - UpdateUserPasswordQuery already migrated (uses connections)
   - **Recommendation:** After Phase 5, refactor to acquire connection in scoped blocks:
     ```rust
     {
         let mut conn = pool.acquire().await?;
         let user = GetUserByIdQuery::run(&mut conn, user_id).await?;
         // ... password verification ...
         let updated_user = UpdateUserPasswordQuery::run(&mut conn, user_id, update_data).await?;
         Ok(updated_user)
     }
     ```
   - **Risk:** LOW - Simple scoping change

2. **UserOrchestrator methods** (user_orchestrator.rs:39-149)
   - Current pattern after Phase 5:
     ```rust
     let user = GetUserByIdQuery::run(pool, user_id).await?;
     let allowed = {
         let mut conn = pool.acquire().await?;
         RoleService::check_user_permission(&mut conn, &user, "can_list_users").await?
     };
     let mut conn = pool.acquire().await?;
     GetAllUsersWithRolesQuery::run(&mut conn).await?;
     ```
   - Problem: GetUserByIdQuery uses pool, then connection is acquired for permission check and released, then connection is acquired again
   - GetUserByIdQuery migrates in Phase 5
   - Other queries (GetAllUsersWithRolesQuery, GetUserByIdWithRoleQuery, DeleteUserByIdQuery) already use connections
   - **Recommendation:** After Phase 5, refactor to use scoped connection blocks:
     ```rust
     let user = {
         let mut conn = pool.acquire().await?;
         GetUserByIdQuery::run(&mut conn, user_id).await?
     };
     let allowed = {
         let mut conn = pool.acquire().await?;
         RoleService::check_user_permission(&mut conn, &user, "can_list_users").await?
     };
     {
         let mut conn = pool.acquire().await?;
         GetAllUsersWithRolesQuery::run(&mut conn).await?
     }
     ```
   - **Risk:** LOW - Only need to add scoped blocks

3. **RoleOrchestrator methods** (role_orchestrator.rs:27-181)
   - Methods affected:
      - `get_all_roles_with_permission_check` (line 15-48)
      - `get_role_by_id_with_permission_check` (line 54-87)
      - `update_role_with_permission_check` (line 93-123)
      - `create_role_with_permission_check` (line 129-181)
   - Current pattern after Phase 5:
     ```rust
     let user = GetUserByIdQuery::run(pool, user_id).await?;
     let can_manage_roles = {
         let mut conn = pool.acquire().await?;
         RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?
     };
     let role = RoleService::get_role_by_id(pool, role_id).await?;
     ```
   - Problem: GetUserByIdQuery uses pool, then connection is acquired for permission check and released, then pool is used again
   - GetUserByIdQuery migrates in Phase 5
   - **Recommendation:** After Phase 5, refactor to use scoped connection blocks:
     ```rust
     let user = {
         let mut conn = pool.acquire().await?;
         GetUserByIdQuery::run(&mut conn, user_id).await?
     };
     let can_manage_roles = {
         let mut conn = pool.acquire().await?;
         RoleService::check_user_permission(&mut conn, &user, "can_manage_roles").await?
     };
     let role = RoleService::get_role_by_id(pool, role_id).await?;
     ```
   - Do NOT change RoleService methods at this point - they can continue to acquire their own connections internally
   - **Risk:** MEDIUM - Orchestrator refactoring needed, service methods left as-is

4. **SiteOrchestrator methods** (site_orchestrator.rs:37-151)
   - Methods affected:
       - `create_site_with_permission_check` (line 24-54)
       - `remove_site_with_permission_check` (line 56-86)
       - `update_site_with_permission_check` (line 88-119)
       - `get_site_details_with_permission_check` (line 121-151)
   - Current pattern after Phase 5:
      ```rust
      let user = GetUserByIdQuery::run(pool, user_id).await?;
      let allowed = {
          let mut conn = pool.acquire().await?;
          RoleService::check_user_permission(&mut conn, &user, "can_create_site").await?
      };
      let site = SiteService::create_site(pool, create_data).await?;
      ```
   - Problem: GetUserByIdQuery uses pool, then connection is acquired for permission check and released, then pool is used again
   - GetUserByIdQuery migrates in Phase 5
   - **Recommendation:** After Phase 5, refactor to use scoped connection blocks:
      ```rust
      let user = {
          let mut conn = pool.acquire().await?;
          GetUserByIdQuery::run(&mut conn, user_id).await?
      };
      let allowed = {
          let mut conn = pool.acquire().await?;
          RoleService::check_user_permission(&mut conn, &user, "can_create_site").await?
      };
      let site = SiteService::create_site(pool, create_data).await?;
      ```
      - Do NOT change SiteService methods at this point - they can continue to acquire their own connections internally
   - **Risk:** MEDIUM - Orchestrator refactoring needed, service methods left as-is

### Review After Phase 9 (UpdateUserQuery migrates):

5. **UserService.update_user** (user_service.rs:304-339)
   - Current pattern:
     ```rust
     let current_user = GetUserByIdQuery::run(pool, user_id).await?;
     // ... validation ...
     if let Some(_existing_user) = GetUserByNameQuery::run(pool, name).await? {
         return Err(UserError::UsernameAlreadyExists(name.to_string()));
     }
     // ... password hashing ...
     UpdateUserQuery::run(pool, update_data).await?
     ```
   - Problem: Three sequential pool calls without releasing connections in between
   - All three queries migrate in different phases (Phase 5, Phase 2, Phase 9)
   - **Recommendation:** After Phase 9, refactor to use scoped connection blocks:
     ```rust
     let current_user = {
         let mut conn = pool.acquire().await?;
         GetUserByIdQuery::run(&mut conn, user_id).await?
     };
     // ... validation ...
     if let Some(_existing_user) = {
         let mut conn = pool.acquire().await?;
         GetUserByNameQuery::run(&mut conn, name).await?
     } {
         return Err(UserError::UsernameAlreadyExists(name.to_string()));
     }
     // ... password hashing ...
     {
         let mut conn = pool.acquire().await?;
         UpdateUserQuery::run(&mut conn, update_data).await?
     }
     ```
   - **Risk:** MEDIUM - Three pool calls to consolidate, careful ordering needed

### Review After Phase 8 (CreateUserQuery migrates):

6. **AuthService.register** (auth_service.rs:114-149)
   - Current pattern:
     ```rust
     let user = UserService::create_user(pool, username, password).await?;
     let user_with_role = GetUserByNameWithRoleQuery::run(pool, username).await?;
     // ... session creation ...
     ```
   - Problem: Two sequential pool calls without releasing connections in between
   - CreateUserQuery migrates in Phase 8
   - GetUserByNameWithRoleQuery migrates in Phase 3
   - SessionService not covered in this migration (not a query object)
   - **Recommendation:** After Phase 8, refactor to use scoped connection blocks:
     ```rust
     let user = UserService::create_user(pool, username, password).await?;
     let user_with_role = {
         let mut conn = pool.acquire().await?;
         GetUserByNameWithRoleQuery::run(&mut conn, username).await?
     };
     // ... session creation ...
     ```
   - **Risk:** LOW - Only need to add scoped block for second query

### Review After Multiple Phases Complete:

7. **AuthOrchestrator.change_authenticated_user_password** (auth_orchestrator.rs:10-40)
   - Current pattern:
     ```rust
     let _user = GetUserByIdQuery::run(pool, user_id).await?;
     UserService::update_password(pool, user_id, current_password, new_password, new_password_confirmation).await
     ```
   - Problem: GetUserByIdQuery uses pool, then UserService.update_password is called (which also uses pool)
   - GetUserByIdQuery migrates in Phase 5
   - UserService.update_password calls GetUserByIdQuery internally (covered in review item #1)
   - **Recommendation:** Review after Phase 9 (after both GetUserByIdQuery and UpdateUserQuery are migrated)
     ```rust
     let user = {
         let mut conn = pool.acquire().await?;
         GetUserByIdQuery::run(&mut conn, user_id).await?
     };
     UserService::update_password(pool, user_id, current_password, new_password, new_password_confirmation).await
     ```
   - **Risk:** MEDIUM - Crosses orchestrator and service layers

8. **GraphQL resolvers** (multiple files)
   - Multiple resolvers pass pool to sequential queries
   - **Recommendation:** After all relevant phases complete, refactor to acquire connection once per resolver method
   - **Risk:** LOW - Well-contained, isolated test patterns
   - Files to review:
     - `graphql/resolvers/role_permissions.rs` (line 31)
     - `graphql/resolvers/user.rs` (line 124)
     - `graphql/resolvers/remove_site.rs` (line 127)
     - `graphql/resolvers/update_site.rs` (lines 164, 334)
     - `graphql/resolvers/site.rs` (line 124)
     - `graphql/resolvers/sites.rs` (lines 105, 106)
     - Plus other resolvers that may call migrated queries

## Summary of Review Queue

**After Phase 5 (Immediate Priority):**
- UserService.update_password
- UserOrchestrator (4 methods)
- RoleOrchestrator (4 methods)
- SiteOrchestrator (4 methods)

**After Phase 9 (Second Priority):**
- UserService.update_user

**After Phase 8 (Third Priority):**
- AuthService.register

**After all simple phases (Final Cleanup):**
- AuthOrchestrator.change_authenticated_user_password
- All GraphQL resolvers calling migrated queries

## Complex Query Dependencies

The following queries have circular or cross-query dependencies with other pool-based queries:

1. **CreateUserQuery ↔ GetDefaultRoleQuery**: CreateUserQuery calls GetDefaultRoleQuery
2. **UpdateUserQuery → GetUserByIdQuery**: UpdateUserQuery calls GetUserByIdQuery

These dependencies require special handling. To avoid temporary code where a query acquires a connection from a pool just to call another pool-based query, these queries must be updated together in isolated phases done **after** all simple queries are completed.

## Simple Queries (Phases 1-7)

These queries do not call other pool-based queries and can be migrated independently.
### Phase 1: Update GetUserByUuidQuery ✅ COMPLETE

**Reason:** Only called by tests, lowest risk, simplest query.

**Files to Modify:**
- `src/queries/users/get_user_by_uuid.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**
- `tests/database_integration_tests.rs` - Update call at line 43
- `src/test_utils/mod.rs` - Update call at line 596
- `src/queries/users/get_user_by_uuid.rs` - Update self test calls (lines 45, 61)

**Test Changes:**
- Update all test cases in `get_user_by_uuid.rs` to acquire and use connections
- Update all callers in tests to use connections

**Status:** Completed on 2025-12-27

### Phase 2: Update GetUserByNameQuery ✅ COMPLETE

**Reason:** Called by UserService in production, simple query, fewer callers than GetUserByIdQuery.

**Files to Modify:**
- `src/queries/users/get_user_by_name.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**
- `src/services/user_service.rs` - Update calls at lines 93, 132, and 322
- `src/queries/users/get_user_by_name.rs` - Update self test calls (lines 45, 79, 80, 81, 96)

**Test Changes:**
- Update all test cases in `get_user_by_name.rs` to acquire and use connections
- Update UserService tests to use connections when calling this query

**Status:** Completed on 2025-12-27

### Phase 3: Update GetUserByNameWithRoleQuery ✅ COMPLETE

**Reason:** Called by AuthService for login, simple query.

**Files to Modify:**
- `src/queries/users/get_user_by_name_with_role.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**
- `src/services/auth_service.rs` - Update calls at lines 75 and 132
- `src/queries/users/get_user_by_name_with_role.rs` - Update self test calls (lines 83, 100, 116, 119, 122, 144)

**Test Changes:**
- Update all test cases in `get_user_by_name_with_role.rs` to acquire and use connections
- Update AuthService tests to use connections when calling this query

**Status:** Completed on 2025-12-27

### Phase 4: Update GetUserByIdWithRoleQuery ✅ COMPLETE

**Reason:** Called by AuthService and UserOrchestrator, simple query.

**Files to Modify:**
- `src/queries/users/get_user_by_id_with_role.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**
- `src/orchestrators/user_orchestrator.rs` - Update call at line 106
- `src/services/auth_service.rs` - Update call at line 178
- `src/queries/users/get_user_by_id_with_role.rs` - Update self test calls (lines 83, 98, 116, 123, 141)

**Test Changes:**
- Update all test cases in `get_user_by_id_with_role.rs` to acquire and use connections
- Update AuthService and UserOrchestrator tests to use connections

**Status:** Completed on 2025-12-27

### Phase 5: Update GetUserByIdQuery

**Reason:** Very widely used, simple query that doesn't call other pool-based queries.

**Files to Modify:**
- `src/queries/users/get_user_by_id.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**
- `src/services/user_service.rs` - Update calls at lines 149, 218, 311, 511, 523, 742, 750
- `src/orchestrators/user_orchestrator.rs` - Update calls at lines 39, 90, 141, 206, 583, 673
- `src/queries/users/update_user.rs` - Update call at line 66
- `src/orchestrators/site_orchestrator.rs` - Update calls at lines 37, 69, 102, 134
- `src/graphql/resolvers/role_permissions.rs` - Update call at line 31
- `src/orchestrators/role_orchestrator.rs` - Update calls at lines 27, 67, 107, 142, 181, 216, 1376
- `src/queries/users/get_user_by_id.rs` - Update self test calls (lines 45, 61)
- `src/orchestrators/auth_orchestrator.rs` - Update call at line 27

**Test Changes:**
- Update all test cases in `get_user_by_id.rs` to acquire and use connections
- Update all caller tests to use connections

**Note:** This query is called by UpdateUserQuery. Since GetUserByIdQuery is being migrated first in this phase, UpdateUserQuery will be able to call it with a connection in a later phase.

**Status:** Completed on 2025-12-27

### Phase 6: Update CreateRoleQuery

**Reason:** Called by RoleService in production, simple query.

**Files to Modify:**
- `src/queries/roles/create_role.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**
- `src/services/role_service.rs` - Update call at line 133
- `src/queries/roles/delete_role.rs` - Update test calls (lines 60, 104)
- `src/test_utils/mod.rs` - Update call at line 186
- `src/queries/roles/create_role.rs` - Update self test calls (6 calls)

**Test Changes:**
- Update all test cases in `create_role.rs` to acquire and use connections
- Update all caller tests to use connections

**Status:** Completed on 2025-12-27

### Phase 7: Update CreateSiteQuery ✅ COMPLETE

**Reason:** Called by SiteService in production, simple query.

**Files to Modify:**
- `src/queries/sites/create_site.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**
- `src/services/site_service.rs` - Update call at line 71
- All test files (26 total calls across multiple files):
  - `src/queries/sites/delete_site.rs` (line 49)
  - `src/queries/sites/update_site.rs` (lines 111, 154, 217)
  - `src/queries/sites/get_site_by_id.rs` (lines 38, 104, 105, 144)
  - `src/orchestrators/site_orchestrator.rs` (lines 317, 353, 392, 446, 499, 539)
  - `src/queries/sites/get_all_sites.rs` (lines 52, 53)
  - `src/queries/sites/create_site.rs` (4 calls)
  - `src/graphql/resolvers/remove_site.rs` (line 127)
  - `src/graphql/resolvers/update_site.rs` (lines 164, 334)
  - `src/graphql/resolvers/site.rs` (line 124)
  - `src/graphql/resolvers/sites.rs` (lines 105, 106)

**Test Changes:**
- Update all test cases in `create_site.rs` to acquire and use connections
- Update all caller tests to use connections

**Status:** Completed on 2025-12-27

## Complex Query Groups (Phases 8-9)

These phases handle queries that call other pool-based queries. They must be updated together in isolated phases to avoid temporary code that acquires connections from pools.

### Phase 8: Update CreateUserQuery and GetDefaultRoleQuery Together ✅ COMPLETE

**Reason:** These queries have a circular dependency: CreateUserQuery calls GetDefaultRoleQuery. Both must be updated together to avoid temporary code where one query acquires a connection from a pool to call the other pool-based query.

**Files to Modify:**
- `src/queries/users/create_user.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`
- `src/queries/roles/get_default_role.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**

For CreateUserQuery:
- `tests/database_integration_tests.rs` - Update call at line 87
- `src/services/role_service.rs` - Update call at line 1042
- `src/services/user_service.rs` - Update call at line 113 and all test calls (15 calls)
- `src/queries/users/update_user.rs` - Update test calls (10 calls)
- `src/queries/users/create_user.rs` - Update self test calls (5 calls)
- `src/test_utils/mod.rs` - Update calls at lines 127 and 165
- `src/graphql/resolvers/user.rs` - Update test call at line 124

For GetDefaultRoleQuery:
- `src/queries/users/create_user.rs` - Update call at lines 25 and 170 (already included above)
- `src/services/role_service.rs` - Update calls at lines 1086 and 1103
- `src/test_utils/mod.rs` - Update calls at lines 103, 141, and 310
- `src/queries/roles/get_default_role.rs` - Update self test calls (lines 45, 61, 70)

**Implementation Strategy:**
1. Update both query signatures to use `&mut SqliteConnection`
2. Update all production callers (RoleService, UserService) to acquire connections and pass them
3. Update all test callers to acquire connections
4. Update both query test modules to use connections
5. **Crucially**: In CreateUserQuery, ensure the call to GetDefaultRoleQuery is made with the same connection (not acquiring a new connection)

**Test Changes:**
- Update all test cases in both `create_user.rs` and `get_default_role.rs` to acquire and use connections
- Update all caller tests to use connections
- Ensure tests that create roles (which CreateUserQuery might query as default) work correctly with connection-based queries

**Status:** Completed on 2025-12-27

### Phase 9: Update UpdateUserQuery

**Reason:** UpdateUserQuery calls GetUserByIdQuery. Since GetUserByIdQuery was migrated in Phase 5, this phase can now update UpdateUserQuery to pass a connection to GetUserByIdQuery instead of a pool.

**Files to Modify:**
- `src/queries/users/update_user.rs` - Change from `&SqlitePool` to `&mut SqliteConnection`

**Callers to Update:**
- `src/services/user_service.rs` - Update call at line 336
- `src/queries/users/update_user.rs` - Update self test calls (lines 140, 175, 210, 247, 282, 316, 339, 369, 403, 441)

**Implementation Strategy:**
1. Update UpdateUserQuery signature to use `&mut SqliteConnection`
2. Update the internal call to GetUserByIdQuery at line 66 to pass `conn` instead of `pool`
3. Update all production callers to acquire connections and pass them
4. Update all test callers to acquire connections

**Note:** This query uses `RETURNING` clause which is SQLite-specific. Ensure this works correctly with connections.

**Test Changes:**
- Update all test cases in `update_user.rs` to acquire and use connections
- Update UserService tests to use connections when calling this query

**Status:** Completed on 2025-12-27

## Migration Complete

All 9 phases have been completed successfully. All query objects have been migrated from using `&SqlitePool` to `&mut SqliteConnection`.

### Summary of Completed Phases:
- **Phase 1**: GetUserByUuidQuery ✅
- **Phase 2**: GetUserByNameQuery ✅
- **Phase 3**: GetUserByNameWithRoleQuery ✅
- **Phase 4**: GetUserByIdWithRoleQuery ✅
- **Phase 5**: GetUserByIdQuery ✅
- **Phase 6**: CreateRoleQuery ✅
- **Phase 7**: CreateSiteQuery ✅
- **Phase 8**: CreateUserQuery + GetDefaultRoleQuery ✅
- **Phase 9**: UpdateUserQuery ✅

### Next Steps (Optional Refactoring):

The following callers can now be reviewed for further optimization using connection consolidation patterns described in this plan:

1. **After Phase 9**: Review UserService.update_user (already consolidated during Phase 9)

2. **After Phase 5**: Review and refactor:
   - UserService.update_password
   - UserOrchestrator methods (4 methods)
   - RoleOrchestrator methods (4 methods)
   - SiteOrchestrator methods (4 methods)

3. **After Phase 8**: Review AuthService.register

4. **After all phases complete**: Review AuthOrchestrator.change_authenticated_user_password and GraphQL resolvers

See the "Callers Requiring Further Review" section in this plan for detailed recommendations on each caller.

## Implementation Guidelines

### Query Object Changes

For each query object being migrated:

1. Change the import from `sqlx::SqlitePool` to `sqlx::SqliteConnection`
2. Update the `run` method signature from `&SqlitePool` to `&mut SqliteConnection`
3. Replace `.execute(pool)` with `.execute(&mut *conn)`
4. Replace `.fetch_one(pool)` with `.fetch_one(&mut *conn)`
5. Replace `.fetch_optional(pool)` with `.fetch_optional(&mut *conn)`
6. Replace `.fetch_all(pool)` with `.fetch_all(&mut *conn)`

**Important:** There are two different patterns for using connections:

1. **sqlx method calls** (`.execute()`, `.fetch_one()`, `.fetch_optional()`, `.fetch_all()`): Always use `&mut *conn` for reborrowing
2. **Calling other query objects**: Always pass `&mut conn` (not `conn` alone)

Example:
```rust
pub async fn run(conn: &mut SqliteConnection, id: i64) -> Result<Option<User>, sqlx::Error> {
    // sqlx method - use &mut *conn
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *conn)  // Reborrowing here
        .await

    // OR call another query - use &mut conn
    // AnotherQuery::run(&mut conn, id).await  // Not reborrowing here
}
```

All callers in the codebase already follow the pattern of passing `&mut conn` to query methods.

### Caller Changes

For each service, orchestrator, and test that calls the query:

1. If the caller already receives a connection, pass it directly
2. If the caller receives a pool, acquire a connection: `let mut conn = pool.acquire().await?;`
3. If tests use the query directly, acquire a connection before calling the query

### Test Changes

For each query's test module:

1. Instead of calling the query with `&pool`, acquire a connection:
   ```rust
   let (pool, _tmp) = create_test_database().await;
   let mut conn = pool.acquire().await.unwrap();
   let result = QueryName::run(&mut conn, params).await.unwrap();
   ```

2. If tests need to create test data before calling the query:
   - First create data using pool-based queries (before acquiring connection)
   - Then acquire connection and call the updated query

## Dependencies

The following dependencies exist between phases:

**Simple Queries (Phases 1-7):**
- Phase 1 (GetUserByUuidQuery) has no dependencies
- Phases 2-4 have no inter-dependencies and can be done in parallel
- Phase 5 (GetUserByIdQuery) must be completed before Phase 9 (UpdateUserQuery) because UpdateUserQuery calls GetUserByIdQuery
- Phase 6 (CreateRoleQuery) has no dependencies
- Phase 7 (CreateSiteQuery) has no dependencies

**Complex Query Groups (Phases 8-9):**
- Phase 8 (CreateUserQuery + GetDefaultRoleQuery) has no dependencies on simple phases - these are updated together in one isolated phase
- Phase 9 (UpdateUserQuery) depends on Phase 5 (GetUserByIdQuery) being completed first

## Completion Criteria

Each phase is complete when:

1. The query object(s) use `&mut SqliteConnection` instead of `&SqlitePool`
2. All production code callers (services, orchestrators, resolvers) have been updated
3. All test code callers have been updated
4. For complex phases: all internal query-to-query calls use connections (no pool-to-connection conversions within queries)
5. All tests in the query's test module pass
6. All tests in caller files pass
7. `cargo test --quiet` passes for the entire codebase

## Notes

- This migration maintains the existing public API of query objects
- Only signature changes from `&SqlitePool` to `&mut SqliteConnection`
- No changes to SQL queries themselves are needed
- Test pool size is set to 1 in integration tests, so connection usage should not cause pool exhaustion issues
- Some queries may need to use `&mut *conn` reborrowing syntax when passing to sqlx methods
- **Crucial**: In complex phases (8-9), ensure that when one pool-based query calls another, they both use the same connection passed in, not acquiring separate connections

## Connection Acquisition Summary

### Key Principles

1. **Never hold a connection when caller is passing a pool to another function or object**
   - With pool size of 1, once a connection is acquired, the pool is empty
   - Any subsequent pool operations will block until the connection is released

2. **Analyze callers on a case-by-case basis**

3. **Never use explicit `drop(conn)` - always use scoped blocks instead**
   - Using `{ let mut conn = ...; ... }` blocks is the preferred pattern for releasing connections
   - Explicit drop statements are harder to read and understand
   - Scoped blocks make the connection lifetime clear and explicit

5. **Three migration cases for callers:**

   **Case 1:** Caller currently does not acquire a connection
   - Acquire connection in a block for the updated query
   - Release connection immediately after query completes
   - Apply to: UserService.create_user, get_user_by_name, get_user_by_id, update_user, AuthService.login, register, get_current_user, RoleService.create_role, SiteService.create_site

   **Case 2:** Caller already acquires a connection
   - Pass that same connection to the updated query
   - No changes needed if caller already follows this pattern
   - Apply to: RoleService methods (get_all_roles, get_role_by_id, etc.), UserService.update_password, UserService.delete_user

   **Case 3:** Caller receives pool as argument but does not use it (after refactoring)
   - After all queries called by the method use connections, acquire connection once at method start
   - Pass that same connection to all queries
   - Remove any blocks that acquire and release connections
   - Apply to: Most orchestrator methods and GraphQL resolvers after all their called queries are migrated

4. **If unsure about a specific caller, list it for further review**
   - See "Callers Requiring Further Review" section above
