# Merge GraphQL Mutation and Query Files into Schema

**Date:** 2025-09-17@18:30  
**Status:** Planning  
**Goal:** Consolidate `src/graphql/mutation.rs` and `src/graphql/query.rs` into `src/graphql/schema.rs` for simplified GraphQL schema management.

## Current State Analysis

The current GraphQL structure is organized as follows:

- `src/graphql/schema.rs`: Contains the schema builder and type definition
- `src/graphql/mutation.rs`: Contains the root `Mutation` struct using `MergedObject`
- `src/graphql/query.rs`: Contains the root `Query` struct using `MergedObject`
- `src/graphql/resolvers/`: Contains individual resolver implementations
- `src/graphql/mod.rs`: Exports all modules

### Current Implementation Details

**Query Structure:**
```rust
#[derive(MergedObject, Default)]
pub struct Query(GetServerTimestampResolver, GetCurrentSessionResolver);
```

**Mutation Structure:**
```rust
#[derive(MergedObject, Default)]
pub struct Mutation(CreateUserResolver, CreateSessionResolver);
```

**Schema Builder:**
```rust
pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;
pub fn build_schema() -> async_graphql::SchemaBuilder<Query, Mutation, EmptySubscription>
```

## Proposed Changes

### 1. Consolidate into schema.rs

Move the `Query` and `Mutation` struct definitions from their separate files into `src/graphql/schema.rs`. This will create a single source of truth for the entire GraphQL schema definition.

### 2. Update Module Structure

- Remove `src/graphql/mutation.rs`
- Remove `src/graphql/query.rs`
- Update `src/graphql/mod.rs` to remove references to the deleted modules
- Keep all resolver imports in `src/graphql/schema.rs`

### 3. Maintain API Compatibility

Ensure that the public API remains unchanged:
- `build_schema()` function signature stays the same
- `AppSchema` type alias remains available
- All existing functionality is preserved

## Implementation Plan

### Phase 1: Move Code to schema.rs

1. **Add resolver imports to schema.rs:**
   ```rust
   use crate::graphql::resolvers::{
       CreateSessionResolver, 
       CreateUserResolver,
       GetCurrentSessionResolver, 
       GetServerTimestampResolver
   };
   ```

2. **Move Query struct to schema.rs:**
   ```rust
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

3. **Move Mutation struct to schema.rs:**
   ```rust
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

### Phase 2: Update Module References

1. **Update src/graphql/mod.rs:**
   ```rust
   pub mod resolvers;
   pub mod schema;
   pub mod types;
   ```

2. **Remove import statements from schema.rs:**
   - Remove `use crate::graphql::mutation::Mutation;`
   - Remove `use crate::graphql::query::Query;`

### Phase 3: Clean Up

1. **Delete files:**
   - `src/graphql/mutation.rs`
   - `src/graphql/query.rs`

2. **Verify compilation:**
   - Run `mise exec -- cargo check`
   - Run `mise exec -- cargo test`

## Files to be Modified

### Created/Modified:
- `src/graphql/schema.rs` - Consolidate all schema definitions
- `src/graphql/mod.rs` - Remove mutation and query module exports

### Deleted:
- `src/graphql/mutation.rs`
- `src/graphql/query.rs`

## Benefits

1. **Single Source of Truth:** All GraphQL schema definitions in one place
2. **Reduced File Count:** Fewer files to maintain and navigate
3. **Improved Cohesion:** Related schema components are co-located
4. **Simplified Imports:** No need to import from separate mutation/query modules

## Risks and Considerations

1. **File Size:** The schema.rs file will be larger, but still manageable given the current scope
2. **Git History:** Moving code will affect git blame, but the benefit outweighs this concern
3. **Future Growth:** If the schema grows significantly, we may need to reconsider this structure

## Testing Strategy

1. **Compilation Test:** Ensure the code compiles without errors
2. **Integration Tests:** Run existing GraphQL integration tests
3. **API Compatibility:** Verify that the public API remains unchanged

## Success Criteria

- [ ] All code compiles successfully
- [ ] All existing tests pass
- [ ] Public API remains unchanged
- [ ] GraphQL schema functionality is preserved
- [ ] Code is properly documented and organized