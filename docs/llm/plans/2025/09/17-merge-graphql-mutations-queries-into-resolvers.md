# Merge GraphQL Mutations and Queries into Single Resolvers Folder

**Date:** 2025-09-17@18:10  
**Status:** Planning  
**Goal:** Consolidate `src/graphql/mutations` and `src/graphql/queries` folders into a single `src/graphql/resolvers` folder and rename classes to use "Resolver" suffix instead of "Query" or "Mutation" suffixes.

## Problem Statement

Currently, the GraphQL structure separates mutations and queries into different folders, and uses "Query" and "Mutation" suffixes for resolver classes. This creates confusion because "query" can refer to multiple concepts (GraphQL queries, database queries, etc.). The goal is to:

1. Merge both folders into a single `src/graphql/resolvers` folder
2. Rename all resolver classes to use "Resolver" suffix instead of "Query" or "Mutation"
3. Update all imports and references accordingly
4. Update documentation to reflect the new structure

## Current Structure Analysis

### Current Files to Migrate:
- `src/graphql/mutations/create_session.rs` → `src/graphql/resolvers/create_session.rs`
- `src/graphql/mutations/create_user.rs` → `src/graphql/resolvers/create_user.rs`
- `src/graphql/queries/get_current_session.rs` → `src/graphql/resolvers/get_current_session.rs`
- `src/graphql/queries/get_server_timestamp.rs` → `src/graphql/resolvers/get_server_timestamp.rs`

### Current Classes to Rename:
- `CreateSessionMutation` → `CreateSessionResolver`
- `CreateUserMutation` → `CreateUserResolver`
- `GetCurrentSessionQuery` → `GetCurrentSessionResolver`
- `GetServerTimestampQuery` → `GetServerTimestampResolver`

### Files to Update:
- `src/graphql/mod.rs` - Update module declarations
- `src/graphql/mutation.rs` - Update imports and struct composition
- `src/graphql/query.rs` - Update imports and struct composition
- `src/graphql/mutations/mod.rs` - Remove (contents moved to resolvers/mod.rs)
- `src/graphql/queries/mod.rs` - Remove (contents moved to resolvers/mod.rs)

## Implementation Plan

### Phase 1: Create New Resolver Structure

1. **Create new resolvers directory and module file**
   - Create `src/graphql/resolvers/` directory
   - Create `src/graphql/resolvers/mod.rs` with exports for all resolvers

2. **Migrate and rename resolver files**
   - Copy `src/graphql/mutations/create_session.rs` to `src/graphql/resolvers/create_session.rs`
   - Copy `src/graphql/mutations/create_user.rs` to `src/graphql/resolvers/create_user.rs`
   - Copy `src/graphql/queries/get_current_session.rs` to `src/graphql/resolvers/get_current_session.rs`
   - Copy `src/graphql/queries/get_server_timestamp.rs` to `src/graphql/resolvers/get_server_timestamp.rs`

3. **Rename resolver classes in new files**
   - In `create_session.rs`: `CreateSessionMutation` → `CreateSessionResolver`
   - In `create_user.rs`: `CreateUserMutation` → `CreateUserResolver`
   - In `get_current_session.rs`: `GetCurrentSessionQuery` → `GetCurrentSessionResolver`
   - In `get_server_timestamp.rs`: `GetServerTimestampQuery` → `GetServerTimestampResolver`

### Phase 2: Update Module System

4. **Update main GraphQL module**
   - Modify `src/graphql/mod.rs` to include `resolvers` module
   - Remove `mutations` and `queries` module declarations

5. **Update root Query and Mutation structs**
   - Modify `src/graphql/query.rs` to import from `resolvers` module
   - Modify `src/graphql/mutation.rs` to import from `resolvers` module
   - Update MergedObject compositions to use new resolver names

### Phase 3: Update Tests and Documentation

6. **Update test imports and references**
   - Update any test files that import the old resolver classes
   - Verify all tests still pass with new naming

7. **Update documentation**
   - Update comments in `src/graphql/schema.rs`
   - Update any documentation in `docs/` that references the old structure
   - Update README.md if it mentions the GraphQL structure

### Phase 4: Cleanup

8. **Remove old directories**
   - Delete `src/graphql/mutations/` directory
   - Delete `src/graphql/queries/` directory

9. **Final verification**
   - Run full test suite to ensure everything works
   - Verify GraphQL schema still builds correctly
   - Test actual GraphQL operations

## Code Samples

### New `src/graphql/resolvers/mod.rs`:
```rust
pub mod create_session;
pub mod create_user;
pub mod get_current_session;
pub mod get_server_timestamp;

pub use create_session::CreateSessionResolver;
pub use create_user::CreateUserResolver;
pub use get_current_session::GetCurrentSessionResolver;
pub use get_server_timestamp::GetServerTimestampResolver;
```

### Updated `src/graphql/mod.rs`:
```rust
pub mod mutation;
pub mod query;
pub mod resolvers;
pub mod schema;
pub mod types;
```

### Updated `src/graphql/mutation.rs`:
```rust
use crate::graphql::resolvers::{CreateSessionResolver, CreateUserResolver};
use async_graphql::MergedObject;

/// Root mutation object for the Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL mutations. It combines all
/// individual mutation resolvers into a single unified interface using MergedObject.
///
/// Available mutations:
/// - createUser: Create a new user account
/// - createSession: Authenticate user and create session (sign-in)
///
/// Future mutations will be added here as the service expands to include
/// user management, authentication, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Mutation(CreateUserResolver, CreateSessionResolver);

impl Mutation {
  pub fn new() -> Self {
    Self(CreateUserResolver, CreateSessionResolver)
  }
}
```

### Updated `src/graphql/query.rs`:
```rust
use crate::graphql::resolvers::{GetCurrentSessionResolver, GetServerTimestampResolver};
use async_graphql::MergedObject;

/// Root query object for the Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL queries. It combines all
/// individual query resolvers into a single unified interface using MergedObject.
///
/// Available queries:
/// - getServerTimestamp: Get current server time for synchronization
/// - getCurrentSession: Get current authenticated user session information
///
/// Future queries will be added here as the service expands to include
/// user authentication, profile management, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Query(GetServerTimestampResolver, GetCurrentSessionResolver);

impl Query {
  pub fn new() -> Self {
    Self::default()
  }
}
```

### Example resolver class rename in `src/graphql/resolvers/create_session.rs`:
```rust
// Change from:
#[derive(Default, Debug)]
pub struct CreateSessionMutation;

#[Object]
impl CreateSessionMutation {
  // ... methods
}

// To:
#[derive(Default, Debug)]
pub struct CreateSessionResolver;

#[Object]
impl CreateSessionResolver {
  // ... methods (unchanged)
}
```

## Files to be Created:
- `src/graphql/resolvers/mod.rs`
- `src/graphql/resolvers/create_session.rs` (migrated from mutations)
- `src/graphql/resolvers/create_user.rs` (migrated from mutations)
- `src/graphql/resolvers/get_current_session.rs` (migrated from queries)
- `src/graphql/resolvers/get_server_timestamp.rs` (migrated from queries)

## Files to be Modified:
- `src/graphql/mod.rs`
- `src/graphql/mutation.rs`
- `src/graphql/query.rs`
- Any test files that import the old resolver classes

## Files to be Deleted:
- `src/graphql/mutations/mod.rs`
- `src/graphql/mutations/create_session.rs`
- `src/graphql/mutations/create_user.rs`
- `src/graphql/queries/mod.rs`
- `src/graphql/queries/get_current_session.rs`
- `src/graphql/queries/get_server_timestamp.rs`

## Testing Strategy

1. **Unit Tests**: Verify all existing unit tests in resolver files continue to pass with new class names
2. **Integration Tests**: Run full test suite to ensure GraphQL schema builds and operations work
3. **Manual Testing**: Test actual GraphQL queries and mutations via the API

## Benefits

1. **Reduced Confusion**: Eliminates ambiguity around "Query" suffix which could refer to GraphQL queries or database queries
2. **Unified Structure**: Single `resolvers` folder is cleaner and more intuitive
3. **Consistent Naming**: All resolvers use the same "Resolver" suffix regardless of operation type
4. **Better Organization**: Easier to find and manage all GraphQL resolvers in one location

## Risks and Mitigation

1. **Breaking Changes**: This is an internal refactor that shouldn't affect the public GraphQL API
2. **Test Failures**: Comprehensive testing will catch any import or naming issues
3. **Documentation Drift**: Plan includes updating all relevant documentation

## Success Criteria

- [ ] All resolver files successfully moved to `src/graphql/resolvers/`
- [ ] All resolver classes renamed to use "Resolver" suffix
- [ ] All imports and module declarations updated
- [ ] All tests pass with new structure
- [ ] GraphQL schema builds and operations work correctly
- [ ] Old mutation/query directories removed
- [ ] Documentation updated to reflect new structure