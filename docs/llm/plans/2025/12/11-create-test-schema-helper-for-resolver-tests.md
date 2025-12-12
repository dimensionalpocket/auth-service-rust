# 11-Create-Test-Schema-Helper-for-Resolver-Tests.md

## Summary

Create a centralized `create_test_schema` helper in `test_utils` to eliminate code duplication and standardize test schema creation across all resolver tests. Currently, 21 resolver files contain 92 instances of `Schema::build` calls with 8 duplicate `TestEmptyQuery` definitions.

## Analysis Report

### Current State
- **21 resolver files** affected with **92 Schema::build calls**
- **8 duplicate TestEmptyQuery definitions** across different files
- **45 EmptyMutation occurrences** with 3 different import patterns
- **97 EmptySubscription occurrences** with 4 different import patterns
- **3 different patterns** for handling empty queries/mutations
- **4 distinct test schema creation patterns** based on context requirements

### Affected Files
All resolver files in `src/graphql/resolvers/`:
1. `server_timestamp.rs` - Query-only, no context
2. `set_default_role.rs` - Mutation-only with TestEmptyQuery
3. `users.rs` - Query-only with database + session
4. `delete_user.rs` - Mutation-only with TestEmptyQuery
5. `add_role.rs` - Mutation-only with TestEmptyQuery
6. `roles.rs` - Query-only with database + session
7. `auth_login.rs` - Mutation-only with config
8. `update_role.rs` - Mutation-only with TestEmptyQuery
9. `role_permissions.rs` - Query-only with database + session
10. `auth_change_password.rs` - Mutation-only with config
11. `auth_me.rs` - Query-only with database + session
12. `site.rs` - Query-only with database + session
13. `user.rs` - Query-only with database + session
14. `role.rs` - Query-only with database + session
15. `sites.rs` - Query-only with database + session
16. `remove_site.rs` - Mutation-only with TestEmptyQuery
17. `add_site.rs` - Mutation-only with TestEmptyQuery
18. `remove_role.rs` - Mutation-only with TestEmptyQuery
19. `auth_register.rs` - Mutation-only with TestEmptyQuery
20. `update_site.rs` - Mutation-only with TestEmptyQuery
21. `auth_logout.rs` - Mutation-only

### Test Schema Creation Patterns Identified
1. **Simple**: No database or context (server_timestamp)
2. **Database-only**: Pool only
3. **Database + Session**: Pool + session context (most common)
4. **Database + Config**: Pool + configuration (auth operations)

## Implementation Plan

### Phase 1: Create Test Schema Helper Methods

**Objective**: Create and test the helper methods in `test_utils`

**Files to modify**:
- `src/test_utils/mod.rs` - Add helper methods

**Helper Methods to Create**:

```rust
// GraphQL test utilities
use async_graphql::{Object, ObjectType, Schema};
use crate::middleware::session::SessionContext;
use dps_config::DpsConfig;

// Re-export GraphQL test utilities for consistent usage
pub use async_graphql::{EmptyMutation, EmptySubscription};

// Centralized TestEmptyQuery to replace all duplicates
#[derive(Default)]
pub struct TestEmptyQuery;

#[Object]
impl TestEmptyQuery {
    async fn dummy(&self) -> &str {
        "test"
    }
}

// For query tests - pass query directly
pub fn create_test_query_schema<Q>(
    query: Q,
    pool: Option<SqlitePool>,
    session: Option<SessionContext>,
    config: Option<DpsConfig>,
) -> Schema<Q, EmptyMutation, EmptySubscription>
where
    Q: ObjectType + 'static,
{
    let mut schema_builder = Schema::build(query, EmptyMutation, EmptySubscription);
    
    if let Some(pool) = pool {
        schema_builder = schema_builder.data(pool);
    }
    
    if let Some(session) = session {
        schema_builder = schema_builder.data(session);
    }
    
    if let Some(config) = config {
        schema_builder = schema_builder.data(config);
    }
    
    schema_builder.finish()
}

// For mutation tests - pass mutation directly
pub fn create_test_mutation_schema<M>(
    mutation: M,
    pool: Option<SqlitePool>,
    session: Option<SessionContext>,
    config: Option<DpsConfig>,
) -> Schema<TestEmptyQuery, M, EmptySubscription>
where
    M: ObjectType + 'static,
{
    let mut schema_builder = Schema::build(TestEmptyQuery, mutation, EmptySubscription);
    
    if let Some(pool) = pool {
        schema_builder = schema_builder.data(pool);
    }
    
    if let Some(session) = session {
        schema_builder = schema_builder.data(session);
    }
    
    if let Some(config) = config {
        schema_builder = schema_builder.data(config);
    }
    
    schema_builder.finish()
}
```

**Testing Strategy**:
- Add tests directly to `src/test_utils/mod.rs` within a `#[cfg(test)]` module
- Test both helper methods with different parameter combinations:
  - For `create_test_query_schema`: pass `TestEmptyQuery` as query parameter
  - For `create_test_mutation_schema`: pass `EmptyMutation` as mutation parameter
  - Test all combinations of pool, session, and config parameters (None/Some)
- Verify schema creation works correctly
- Test basic schema introspection to ensure schemas are valid
- Note: Actual GraphQL execution testing will be covered by existing resolver tests

### Phase 2: Update Single Query Resolver Test

**Objective**: Test the helper with a real query resolver

**Target File**: `src/graphql/resolvers/server_timestamp.rs`
- **Reason**: Simplest case - no database or context required
- **Current Pattern**: `Schema::build(query, EmptyMutation, EmptySubscription).finish()`
- **New Pattern**: `create_test_query_schema(query, None, None, None)`

**Steps**:
1. Update imports in `server_timestamp.rs` to include helper methods
2. Replace `Schema::build` call with `create_test_query_schema`
3. Remove any local `TestEmptyQuery` if present
4. Run tests in isolation: `cargo test server_timestamp --quiet`
5. Fix any issues with helper method
6. Iterate until tests pass

### Phase 3: Update Single Mutation Resolver Test with Data

**Objective**: Test the helper with a mutation requiring database and session

**Target File**: `src/graphql/resolvers/add_site.rs`
- **Reason**: Complex case - mutation with database + session context
- **Current Pattern**: Uses local `TestEmptyQuery` with database + session
- **New Pattern**: `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`

**Steps**:
1. Update imports in `add_site.rs` to include helper methods
2. Remove local `TestEmptyQuery` definition
3. Replace `Schema::build` call with `create_test_mutation_schema`
4. Run tests in isolation: `cargo test add_site --quiet`
5. Fix any issues with helper method
6. Iterate until tests pass

### Phase 4: Apply Changes to All Remaining Resolver Tests

**Objective**: Migrate all remaining resolver files to use helper methods

**Files to Update** (19 remaining files):
1. `set_default_role.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`
2. `users.rs` - Use `create_test_query_schema(query, Some(pool), Some(session), None)`
3. `delete_user.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`
4. `add_role.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`
5. `roles.rs` - Use `create_test_query_schema(query, Some(pool), Some(session), None)`
6. `auth_login.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), Some(config))`
7. `update_role.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`
8. `role_permissions.rs` - Use `create_test_query_schema(query, Some(pool), Some(session), None)`
9. `auth_change_password.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), Some(config))`
10. `auth_me.rs` - Use `create_test_query_schema(query, Some(pool), Some(session), None)`
11. `site.rs` - Use `create_test_query_schema(query, Some(pool), Some(session), None)`
12. `user.rs` - Use `create_test_query_schema(query, Some(pool), Some(session), None)`
13. `role.rs` - Use `create_test_query_schema(query, Some(pool), Some(session), None)`
14. `sites.rs` - Use `create_test_query_schema(query, Some(pool), Some(session), None)`
15. `remove_site.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`
16. `remove_role.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`
17. `auth_register.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`
18. `update_site.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`
19. `auth_logout.rs` - Use `create_test_mutation_schema(mutation, Some(pool), Some(session), None)`

**Migration Strategy**:
1. Process files in logical groups (queries first, then mutations)
2. For each file:
   - Update imports to include helper methods (remove EmptyMutation and EmptySubscription imports)
   - Remove local `TestEmptyQuery` definitions
   - Replace `Schema::build` calls with appropriate helper method
   - Run tests to verify
3. After each group, run full test suite: `cargo test --quiet`
4. Fix any issues that arise

### Phase 5: Update AGENTS.md Documentation

**Objective**: Document the new schema helper in AGENTS.md for future reference

**Files to modify**:
- `AGENTS.md` - Add schema helper documentation under Testing section

**Documentation Preview**:
````markdown
### GraphQL Test Schema Helper

When writing resolver tests, use centralized `create_test_schema` helper from `test_utils` instead of direct `Schema::build` calls:

```rust
use crate::test_utils::{create_test_query_schema, create_test_mutation_schema, TestEmptyQuery};

// Query-only test (no context)
let schema = create_test_query_schema(query, None, None, None);

// Mutation with database and session
let schema = create_test_mutation_schema(mutation, Some(pool), Some(session), None);

// Auth mutation with database, session, and config
let schema = create_test_mutation_schema(mutation, Some(pool), Some(session), Some(config));
```

**Note**: The helper automatically provides default empty types when `None` is passed for query/mutation/subscription parameters.
````

## Expected Benefits

- Eliminate 8 duplicate `TestEmptyQuery` definitions
- Standardize 45 `EmptyMutation` imports across 3 different patterns
- Standardize 97 `EmptySubscription` imports across 4 different patterns
- Reduce 92 `Schema::build` calls to standardized helper calls
- Improve test consistency and maintainability
- Centralize test schema creation logic

## Implementation Details

### Required Imports
The helper methods will require these imports in `test_utils/mod.rs`:
```rust
use async_graphql::{Object, ObjectType, Schema};
use crate::middleware::session::SessionContext;
use dps_config::DpsConfig;

// Re-export GraphQL test utilities for consistent usage
pub use async_graphql::{EmptyMutation, EmptySubscription};
```

### Error Handling
- Helper methods return `Schema<...>` directly (no Result needed)
- Schema::build() will panic if there are configuration issues, which is appropriate for tests

### Generic Constraints
- Query types: `Q: ObjectType + 'static`
- Mutation types: `M: ObjectType + 'static`
- Parameters take owned values (not references) to avoid lifetime issues:
  - `pool: Option<SqlitePool>`
  - `session: Option<SessionContext>`
  - `config: Option<DpsConfig>`

### Testing Requirements
- Helper method must be thoroughly tested with different parameter combinations
- Test both success and error scenarios
- Verify GraphQL schema functionality
- Ensure compatibility with existing test patterns

## Success Criteria

1. **Phase 1**: Two helper methods created and tested
2. **Phase 2**: `server_timestamp.rs` successfully migrated
3. **Phase 3**: `add_site.rs` successfully migrated
4. **Phase 4**: All 21 resolver files successfully migrated
5. **Phase 5**: Update AGENTS.md with schema helper documentation
6. **Final**: All resolver tests pass with new helper methods
7. **Cleanup**: No duplicate `TestEmptyQuery` definitions remain
8. **Cleanup**: Standardized EmptyMutation imports across all files
9. **Cleanup**: Standardized EmptySubscription imports across all files
10. **Verification**: Full test suite passes: `cargo test --quiet`