# Refactor GraphQL Schema Creation to Use Builder Pattern

**Date**: 2025-01-27@15:30

## Overview

Refactor the GraphQL schema creation from a factory pattern to a builder pattern by renaming `create_schema()` to `build_schema()` and returning a schema builder instead of a finished schema. This will allow flexible data injection while maintaining clean separation between database-agnostic and database-dependent use cases.

## Current State

- `create_schema()` returns a finished `AppSchema` with no data context
- Production code uses `create_schema()` but mutations fail due to missing database pool
- Tests manually create schemas with database pool injection
- GraphQL handlers expect database pool in schema context

## Target State

- `build_schema()` returns a `SchemaBuilder` that can be customized
- Production code uses `build_schema().data(pool).finish()` for database-dependent operations
- Tests use `DpAuthServer::create_app()` instead of manual schema creation
- GraphQL Playground uses `build_schema().finish()` for database-agnostic introspection

## Implementation Plan

### Phase 1: Rename and Update Schema Creation

#### Step 1A: Update `src/graphql/schema.rs`
```rust
// Before:
pub fn create_schema() -> AppSchema {
    Schema::build(Query::new(), Mutation::new(), EmptySubscription).finish()
}

// After:
pub fn build_schema() -> SchemaBuilder<Query, Mutation, EmptySubscription> {
    Schema::build(Query::new(), Mutation::new(), EmptySubscription)
}
```

#### Step 1B: Update Documentation
- Update function documentation to reflect builder pattern
- Add examples showing both `.finish()` and `.data(...).finish()` usage

### Phase 2: Update Production Code

#### Step 2A: Update `src/dp_auth_server.rs`
```rust
// In create_app() method:
pub async fn create_app(&self) -> Result<Router, DpAuthServerError> {
    let database = self.initialize_database().await?;
    let schema = crate::graphql::schema::build_schema()
        .data(database.pool)
        .finish();
    Ok(self.build_router(schema))
}
```

#### Step 2B: Update GraphQL Handlers in `src/handlers/graphql.rs`
- For GET requests (GraphQL Playground): Use `build_schema().finish()`
- For POST requests: Schema should come from router state (already has database)

### Phase 3: Update All Other Usages

#### Step 3A: Update `src/lib.rs` (Old API)
```rust
// Replace:
let schema = graphql::schema::create_schema();

// With:
let schema = graphql::schema::build_schema().finish();
```

#### Step 3B: Update Unit Tests
Find all unit tests that use `create_schema()` and update them:

**Database-independent tests:**
```rust
// Replace:
let schema = create_schema();

// With:
let schema = build_schema().finish();
```

**Database-dependent tests:**
```rust
// Replace:
let schema = Schema::build(Query::new(), Mutation::new(), EmptySubscription)
    .data(pool)
    .finish();

// With:
let schema = build_schema()
    .data(pool)
    .finish();
```

### Phase 4: Update Integration Tests

#### Step 4A: Simplify `create_app_with_database()` in `tests/integration_tests.rs`
```rust
// Replace current manual schema creation:
async fn create_app_with_database() -> (Router, SqlitePool, tempfile::NamedTempFile) {
    // ... server setup ...
    
    // Remove manual schema creation:
    // let schema = Schema::build(Query::new(), Mutation::new(), EmptySubscription)
    //     .data(database.pool.clone())
    //     .finish();
    // let app = server.build_router(schema);
    
    // Use server's create_app method instead:
    let app = server.create_app().await.expect("Failed to create app");
    
    (app, database.pool, temp_file)
}
```

#### Step 4B: Update `create_app()` Helper
```rust
// Replace:
let server = DpAuthServer::new()
    .session_secret(vec![0u8; 32])
    .build()
    .unwrap();
let app = server.create_app();

// With:
let server = DpAuthServer::new()
    .session_secret(vec![0u8; 32])
    .build()
    .unwrap();
let app = server.create_app().await.expect("Failed to create app");
```

### Phase 5: Update Method Signatures

#### Step 5A: Make `create_app()` Async
Since `create_app()` now needs to initialize the database, it must be async:

```rust
// Before:
pub fn create_app(&self) -> Router

// After:
pub async fn create_app(&self) -> Result<Router, DpAuthServerError>
```

#### Step 5B: Update All Callers
Update all places that call `create_app()` to handle the async nature and error result.

## Files to Modify

### Core Files
1. **`src/graphql/schema.rs`**
   - Rename `create_schema()` to `build_schema()`
   - Change return type from `AppSchema` to `SchemaBuilder<Query, Mutation, EmptySubscription>`
   - Update documentation

2. **`src/dp_auth_server.rs`**
   - Update `create_app()` to be async and return `Result<Router, DpAuthServerError>`
   - Use `build_schema().data(database.pool).finish()`

3. **`src/handlers/graphql.rs`**
   - Update GraphQL GET handler to use `build_schema().finish()`

4. **`src/lib.rs`**
   - Update old API to use `build_schema().finish()`

### Test Files
5. **`tests/integration_tests.rs`**
   - Update `create_app_with_database()` to use `server.create_app().await`
   - Update `create_app()` helper to handle async
   - Remove manual schema creation

6. **Unit test files** (find with grep):
   - `src/graphql/mutations/create_user.rs`
   - `src/graphql/mutations/create_session.rs`
   - Any other files using `create_schema()`

## Search and Replace Strategy

### Step 1: Find All Usages
```bash
grep -r "create_schema" src/ tests/ --include="*.rs"
```

### Step 2: Categorize Replacements
- **Database-independent**: `create_schema()` → `build_schema().finish()`
- **Database-dependent**: Manual schema building → `build_schema().data(pool).finish()`
- **Production**: Use `server.create_app().await` instead of manual schema creation

### Step 3: Update Imports
Update any imports that reference `create_schema` to `build_schema`.

## Testing Strategy

1. **Unit Tests**: Ensure all unit tests pass with new schema builder pattern
2. **Integration Tests**: Verify integration tests work with `server.create_app().await`
3. **Manual Testing**: Test GraphQL Playground still works (GET requests)
4. **Mutation Testing**: Verify mutations work in both tests and production

## Benefits

1. **Fixes Production Bug**: Database pool will be properly available to mutations
2. **Unifies Test and Production**: Both use the same schema creation pattern
3. **Maintains Flexibility**: Can still create database-agnostic schemas for tooling
4. **Cleaner Architecture**: Server manages database lifecycle, schema gets configured database
5. **Better Separation**: Clear distinction between builder (flexible) and finished schema (immutable)

## Breaking Changes

1. **`create_schema()` renamed to `build_schema()`**: All callers must update
2. **`create_app()` becomes async**: All callers must handle async and error result
3. **Schema builder pattern**: Manual schema creation code needs updates

## Migration Path

1. Implement changes in order (schema → production → tests)
2. Run tests after each phase to catch issues early
3. Update documentation and examples
4. Consider this a patch release since it fixes a production bug

This plan will unify the schema creation pattern across the codebase while fixing the production database access issue and simplifying test code.