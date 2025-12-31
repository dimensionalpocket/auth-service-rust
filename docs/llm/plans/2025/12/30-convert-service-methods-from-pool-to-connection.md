# Plan: Convert Service Methods from Pool to Connection

**Date:** 2025-12-30

## Overview

Convert all service methods that currently accept a `SqlitePool` to accept `&mut SqliteConnection` instead. The orchestrators will be responsible for acquiring the connection and passing it to all service methods and queries within the orchestrator method.

## Goals

- Orchestrators will acquire connections and pass the same connection to all service methods and queries inside that orchestrator method
- At the end of this refactor, pools should no longer be passed to other methods inside orchestrators
- Connections should not be scoped in blocks
- Service methods should not deal with pools at all (they will only accept connections)
- For tests, methods using "with_pool" should be updated to use connection versions if available
- For integration tests, connections may need to remain scoped in blocks when calling the schema

---

## Phase 0: Analysis Complete

### Summary of Findings

Total service methods accepting pool: 18

### Service Method Details

| # | Service | Method | Passes Pool To | Count of Callers | Priority |
|---|----------|--------|------------------|------------------|-----------|
| 1 | AuthService | `login` | Yes (SessionService::create_session) | 2 | High |
| 2 | AuthService | `register` | Yes (UserService::create_user) | 1 | High |
| 3 | AuthService | `get_current_user` | No | 1 | Low |
| 4 | RoleService | `get_all_roles` | No | 5 | Low |
| 5 | RoleService | `get_role_by_id` | No | 7 | Low |
| 6 | RoleService | `get_role_by_name` | No | 1 | Low |
| 7 | RoleService | `create_role` | No | 4 | Low |
| 8 | RoleService | `delete_role` | No | 3 | Low |
| 9 | RoleService | `update_role` | No | 2 | Low |
| 10 | RoleService | `set_default_role` | No | 3 | Low |
| 11 | RoleService | `check_user_permission` | N/A (already uses conn) | 18 | N/A |
| 12 | SessionService | `create_session` | Yes (UserService::get_user_by_name) | 2 | High |
| 13 | SessionService | `create_session_for_user` | No | 3 | Low |
| 14 | SiteService | `create_site` | No | 3 | Low |
| 15 | SiteService | `get_all_sites` | No | 1 | Low |
| 16 | SiteService | `update_site` | No | 1 | Low |
| 17 | SiteService | `delete_site` | No | 2 | Low |
| 18 | UserService | `create_user` | No | 22 | Low |
| 19 | UserService | `get_user_by_name` | No | 2 | Low |
| 20 | UserService | `get_user_by_id` | No | 3 | Low |
| 21 | UserService | `update_password` | No | 7 | Low |
| 22 | UserService | `delete_user` | No | 4 | Low |
| 23 | UserService | `update_user` | No | 11 | Low |

**Note:** `RoleService::check_user_permission` already accepts `&mut SqliteConnection` - no conversion needed.

### Detailed Caller Analysis

#### Method 1: `AuthService::login(pool, ...)`
- **Passes pool to:** `SessionService::create_session(pool, ...)`
- **Callers:**
  - `src/graphql/resolvers/auth_login.rs:46`
  - `src/graphql/resolvers/auth_change_password.rs:117` (test setup)
- **Test impact:** `src/services/auth_service.rs` tests
- **Priority:** High (passes pool to another service)

#### Method 2: `AuthService::register(pool, ...)`
- **Passes pool to:** `UserService::create_user(pool, ...)`
- **Callers:**
  - `src/graphql/resolvers/auth_register.rs:63`
- **Test impact:** `src/services/auth_service.rs` tests
- **Priority:** High (passes pool to another service)

#### Method 3: `AuthService::get_current_user(pool, ...)`
- **Passes pool to:** None (acquires connection for query)
- **Callers:**
  - `src/graphql/resolvers/auth_me.rs:119`
- **Test impact:** `src/services/auth_service.rs` tests
- **Priority:** Low (simple, no pool passing)

#### Method 4: `RoleService::get_all_roles(pool)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/role_orchestrator.rs:50`
  - Tests in `src/services/role_service.rs` (5 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 5: `RoleService::get_role_by_id(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/role_orchestrator.rs:90`
  - Tests in `src/services/role_service.rs` (6 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 6: `RoleService::get_role_by_name(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - Test in `src/services/role_service.rs:317`
- **Test impact:** Service tests
- **Priority:** Low

#### Method 7: `RoleService::create_role(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/role_orchestrator.rs:179`
  - Tests in `src/services/role_service.rs` (4 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 8: `RoleService::delete_role(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/role_orchestrator.rs:219`
  - Tests in `src/services/role_service.rs` (3 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 9: `RoleService::update_role(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/role_orchestrator.rs:135`
  - Tests in `src/services/role_service.rs` (2 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 10: `RoleService::set_default_role(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/role_orchestrator.rs:259`
  - Tests in `src/services/role_service.rs` (3 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 11: `RoleService::check_user_permission(conn, ...)`
- **Already accepts:** `&mut SqliteConnection` (no conversion needed)
- **Callers:** 18 orchestrator calls across 3 orchestrator files

#### Method 12: `SessionService::create_session(pool, ...)`
- **Passes pool to:** `UserService::get_user_by_name(pool, ...)`
- **Callers:**
  - `src/services/auth_service.rs:91` (from AuthService::login)
  - Test in `src/services/auth_service.rs:218`
- **Test impact:** `src/services/session_service.rs` tests
- **Priority:** High (passes pool to another service)

#### Method 13: `SessionService::create_session_for_user(...)`
- **Passes pool to:** None (no pool parameter, takes User directly)
- **Callers:**
  - `src/services/auth_service.rs:91` (from AuthService::login)
  - `src/services/auth_service.rs:146` (from AuthService::register)
  - Tests in `src/services/session_service.rs`
- **Priority:** Low (no pool parameter to convert, but used by services that do)

#### Method 14: `SiteService::create_site(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/site_orchestrator.rs:56`
  - Tests in `src/services/site_service.rs` (2 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 15: `SiteService::get_all_sites(pool)`
- **Passes pool to:** None
- **Callers:**
  - `src/graphql/resolvers/sites.rs:45`
  - Test in `src/services/site_service.rs`
- **Test impact:** Service tests
- **Priority:** Low

#### Method 16: `SiteService::update_site(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/site_orchestrator.rs:127`
  - Tests in `src/services/site_service.rs`
- **Test impact:** Service tests
- **Priority:** Low

#### Method 17: `SiteService::delete_site(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/site_orchestrator.rs:91`
  - Tests in `src/services/site_service.rs` (2 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 18: `UserService::create_user(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/services/auth_service.rs:135` (from AuthService::register)
  - `src/orchestrators/user_orchestrator.rs:223`
  - Tests in `src/services/user_service.rs` (17 occurrences)
  - Tests in `src/graphql/resolvers/auth_login.rs:113`
  - Tests in `src/graphql/resolvers/auth_change_password.rs:112`
- **Test impact:** Service tests + resolver tests
- **Priority:** Low

#### Method 19: `UserService::get_user_by_name(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/services/session_service.rs:114` (from SessionService::create_session)
  - Tests in `src/services/user_service.rs` (2 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 20: `UserService::get_user_by_id(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/user_orchestrator.rs:223`
  - Tests in `src/services/user_service.rs` (3 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 21: `UserService::update_password(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/auth_orchestrator.rs:35`
  - Tests in `src/services/user_service.rs` (7 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 22: `UserService::delete_user(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/user_orchestrator.rs:160`
  - Tests in `src/services/user_service.rs` (4 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

#### Method 23: `UserService::update_user(pool, ...)`
- **Passes pool to:** None
- **Callers:**
  - `src/orchestrators/user_orchestrator.rs:223`
  - Tests in `src/services/user_service.rs` (11 occurrences)
- **Test impact:** Service tests
- **Priority:** Low

---

## Phases

### Phase 1: Learning Phase - Single Method (Priority: Low)

**Goal:** Learn the conversion pattern with the simplest method.

**Method to convert:** `AuthService::get_current_user`

**Rationale:**
- Only 1 caller (`auth_me.rs`)
- Does NOT pass pool to other methods (only acquires connection for query)
- Simple implementation - good learning opportunity
- Already demonstrates the pattern we want (acquire connection, pass to query)

**Implementation:**
1. Change `AuthService::get_current_user(pool: &SqlitePool, ...)` to `get_current_user(conn: &mut SqliteConnection, ...)`
2. Update the caller `src/graphql/resolvers/auth_me.rs:119` to acquire connection before calling
3. Update tests in `src/services/auth_service.rs`

**Files to modify:**
- `src/services/auth_service.rs`
- `src/graphql/resolvers/auth_me.rs`

**Test approach:** Run `AuthService::get_current_user` tests, then `auth_me` resolver tests, then all tests to ensure no regressions.

---

### Phase 2: SiteService Methods (Priority: Low)

**Methods to convert:**
1. `SiteService::get_all_sites`
2. `SiteService::create_site`
3. `SiteService::update_site`
4. `SiteService::delete_site`

**Rationale:** All simple methods, low caller count, no pool passing

**Files to modify:**
- `src/services/site_service.rs`
- `src/orchestrators/site_orchestrator.rs` (4 orchestrator methods acquire connections)
- `src/graphql/resolvers/sites.rs`
- Tests in `src/services/site_service.rs`

---

### Phase 3: RoleService Methods - Part 1 (Priority: Low)

**Methods to convert:**
1. `RoleService::get_role_by_name`
2. `RoleService::get_role_by_id`
3. `RoleService::get_all_roles`

**Rationale:** Simple getters, no pool passing

**Files to modify:**
- `src/services/role_service.rs`
- `src/orchestrators/role_orchestrator.rs` (2 orchestrator methods acquire connections)
- Tests in `src/services/role_service.rs`

---

### Phase 4: RoleService Methods - Part 2 (Priority: Low)

**Methods to convert:**
1. `RoleService::create_role`
2. `RoleService::update_role`
3. `RoleService::set_default_role`
4. `RoleService::delete_role`

**Rationale:** All CRUD operations, moderate complexity

**Files to modify:**
- `src/services/role_service.rs`
- `src/orchestrators/role_orchestrator.rs` (4 orchestrator methods acquire connections)
- Tests in `src/services/role_service.rs`

---

### Phase 5: UserService Methods - Part 1: Simple Queries (Priority: Low)

**Methods to convert:**
1. `UserService::get_user_by_name`
2. `UserService::get_user_by_id`

**Rationale:** Simple query methods, low caller count

**Files to modify:**
- `src/services/user_service.rs`
- `src/services/session_service.rs` (calls get_user_by_name)
- Tests in `src/services/user_service.rs`

---

### Phase 6: UserService Methods - Part 2: CRUD Operations (Priority: Low)

**Methods to convert:**
1. `UserService::delete_user`
2. `UserService::update_password`
3. `UserService::update_user`

**Rationale:** CRUD operations with moderate caller count

**Files to modify:**
- `src/services/user_service.rs`
- `src/orchestrators/user_orchestrator.rs` (3 orchestrator methods acquire connections)
- `src/orchestrators/auth_orchestrator.rs` (1 orchestrator method)
- Tests in `src/services/user_service.rs`

---

### Phase 7: Complex Auth Methods (Priority: High)

**Methods to convert:**
1. `SessionService::create_session`
2. `AuthService::login` (depends on SessionService::create_session)
3. `AuthService::register` (depends on UserService::create_user)

**Rationale:** These methods pass pools to other services, requiring coordination

**Order within phase:**
1. First convert `SessionService::create_session` (requires also converting `UserService::get_user_by_name`)
2. Then convert `AuthService::login`
3. Then convert `AuthService::register`
4. Then update `create_authenticated_session` test helper in `src/graphql/resolvers/auth_change_password.rs` to take connection instead of pool, since all service methods it calls will now use connections

**Files to modify:**
- `src/services/session_service.rs`
- `src/services/user_service.rs`
- `src/services/auth_service.rs`
- Tests in `src/services/session_service.rs`
- Tests in `src/services/auth_service.rs`
- `src/graphql/resolvers/auth_login.rs`
- `src/graphql/resolvers/auth_register.rs`
- `src/graphql/resolvers/auth_change_password.rs` (update `create_authenticated_session` test helper to take connection)

---

## Important Implementation Notes

### Orchestrator Pattern
Follow the pattern in `RoleOrchestrator::get_all_role_permissions_with_permission_check`:
```rust
let mut conn = pool.acquire().await.map_err(ServiceError::DatabaseError)?;

// Use the same conn for all service calls and queries in this orchestrator
SomeService::method(&mut conn, ...).await?
SomeQuery::run(&mut conn, ...).await?
```

### Connection Scope
- **Do NOT** scope connections in blocks inside orchestrators
- Acquire connection before it's first needed in the orchestrator method
- Pass the same connection to ALL service methods and queries
- Let the connection be dropped at the end of the orchestrator method (do not use `drop(conn)`)

### Testing Considerations

#### Service Tests
- For service tests using `create_test_database()`, acquire connection once at test start and reuse
- Remove `pool.acquire()` calls from service methods themselves
- Example pattern:
```rust
#[tokio::test]
async fn test_something() {
    let (pool, _temp_file) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();
    
    // Now call service methods with conn
    let result = SomeService::method(&mut conn, ...).await;
    
    assert!(result.is_ok());
}
```

#### Orchestrator Tests
- Pool is still needed (orchestrator tests call orchestrators which use pool)
- Connection may need to be scoped in blocks when calling anything that uses pool, such as orchestrator methods, the schema, or a test helper that uses the schema
- For direct orchestrator method calls, when creating test data, scope the connections so that the pool is usable when the orchestrator method is called in the test

#### Test Utils
- If a test method uses `create_test_*_with_pool`, and a connection-based version exists, update to use it
- Connection-based versions available: `create_test_user`, `create_test_user_full`, `create_test_role_model`
- Pool-based versions: `create_test_user_with_pool`, `create_test_user_full_with_pool`, `create_test_role_model_with_pool`
- Test helper methods (like `create_authenticated_session` in `auth_change_password.rs`) that are used across multiple tests should be updated to accept connections when all their internal calls use connections

#### Integration Tests
- When calling schema (GraphQL), the pool is used (tests use a pool size of 1)
- In such cases, connections may need to remain scoped in blocks when creating test data before schema calls

### Integration with Existing Patterns

- `RoleService::check_user_permission` already uses `&mut SqliteConnection` - this is our reference pattern
- Orchestrators like `get_all_role_permissions_with_permission_check` demonstrate the ideal workflow

---

## Testing Strategy

After each phase:
1. Run **all tests** to ensure no regressions across the entire codebase: `cargo test --quiet`
2. Run linter: `cargo clippy --allow-dirty --fix && cargo fmt`

---

## Unforeseen Issues

If any phase introduces issues that are too complex to address, STOP and let the user know what is happening.

---

## Success Criteria

Each phase is complete when:
1. All service methods in the phase accept `&mut SqliteConnection` instead of `&SqlitePool`
2. All callers (orchestrators, tests, etc) are updated to acquire and pass connections
3. **All tests** pass
4. No lint errors or warnings
5. The connection is acquired once per orchestrator method and reused throughout

---

## Known Risks and Mitigations

### Risk: Test utility methods may need updates
**Mitigation:** Check test_utils methods that create test data. If they use `*_with_pool` methods, see if a version without the pool suffix exists.

### Risk: Integration tests may need different handling
**Mitigation:** When calling the schema (GraphQL), the schema manages connections. But orchestrator methods should still follow the connection pattern.

### Risk: Methods passing pools to other services need coordination
**Mitigation:** Phase 7 handles these methods together to ensure proper dependency order.

### Risk: Circular dependencies if services call each other
**Mitigation:** Orchestrators manage the flow, services only accept connections. The orchestrator acquires once and passes to all.

---

## Timeline Estimate

- Phase 1: 1-2 hours (learning phase)
- Phase 2: 2-3 hours
- Phase 3: 2-3 hours
- Phase 4: 3-4 hours
- Phase 5: 2-3 hours
- Phase 6: 3-4 hours
- Phase 7: 4-5 hours

**Total estimated time:** 17-24 hours
