# Multi-Database Support for GraphQL Context

## Date
2026-01-03

## Summary
Analysis of current database pool usage in GraphQL contexts and plan for supporting multiple SQLite database instances. All database instances should be accessible to all resolvers in a multi-threaded/async scenario.

## Current State Analysis

### Database Module Layout

- `src/database/mod.rs` is now a thin module that re-exports `Database`.
- `Database` itself lives in `src/database/database.rs`.

### Database Pool Injection Flow

1. **Database Initialization** (`src/database/database.rs`):
   - `Database` struct contains a single `pub pool: SqlitePool`
   - Pool is created via `Database::new()` or `Database::new_with_pool_size()`
   - Pool is configured with SQLite PRAGMA settings (WAL mode, foreign keys, etc.)

2. **Schema Data Injection** (`src/dps_auth_api.rs:137-143`):
   ```rust
   let database = self.initialize_database().await?;
   let schema = crate::graphql::schema::build_schema()
     .data(database.pool)  // Single pool injected here
     .data(self.config.as_ref().clone())
     .extension(async_graphql::extensions::Tracing)
     .finish();
   ```
   - Only ONE `SqlitePool` is injected into GraphQL schema data
   - This happens at application startup (not per-request)

3. **Request-Level Context** (`src/handlers/graphql.rs:16-27`):
   ```rust
   pub async fn graphql_handler(
     State(schema): State<AppSchema>,
     Extension(session): Extension<SessionContext>,
     req: GraphQLRequest,
   ) -> GraphQLResponse {
     let request = req.into_inner().data(session);  // Session added here
     schema.execute(request).await.into()
   }
   ```
   - `SessionContext` is injected per-request
   - Database pool is already available from schema state

### Database Pool Retrieval Patterns

1. **Resolver Retrieval** (all resolvers):
   ```rust
   let pool = ctx.data::<SqlitePool>()?;
   ```
   Used in:
   - `src/graphql/resolvers/add_site.rs:76`
   - `src/graphql/resolvers/users.rs:49`
   - `src/graphql/resolvers/auth_login.rs:35`
   - And all other resolvers that need database access

2. **Orchestrator Passing** (all orchestrators):
   ```rust
   pub async fn run(
     pool: &SqlitePool,  // Pool passed as parameter
     session_context: SessionContext,
     create_data: CreateSiteData,
   ) -> Result<crate::models::Site, SiteError> {
     let mut conn = pool.acquire().await?;  // Extract connection
     // ... use connection
   }
   ```

3. **Connection Extraction** (orchestrators):
   ```rust
   let mut conn = pool.acquire().await?;
   ```
   - Each orchestrator extracts a single connection from the pool
   - Services receive `&mut SqliteConnection` (not the pool)

### Thread Safety & Concurrency

**Current Behavior**:
- `SqlitePool` from sqlx is thread-safe and async-compatible
- `SqliteConnection` is NOT thread-safe (borrowed from pool)
- Each request gets its own connection from the pool
- Multiple concurrent requests can execute in parallel
- SQLite WAL mode enables multiple readers + single writer

**Test Utilities** (`src/test_utils/mod.rs`):
- `create_test_query_schema()` / `create_test_mutation_schema()` inject pool into test schemas
- Tests create single pool per test (no multiple pools in same test)

## Multi-Database Design

### Requirements
1. Support multiple SQLite database instances (current + at least one more, potentially many)
2. All databases accessible to all resolvers
3. Maintain thread safety and async compatibility
4. Minimize code changes across existing resolvers/orchestrators

### Proposed Solution: DatabaseRegistry

Create a `DatabaseRegistry` struct that manages multiple named database pools:

```rust
// src/database/database_registry.rs - New module

/// Registry for managing multiple database pools
#[derive(Clone)]
pub struct DatabaseRegistry {
  pools: HashMap<String, SqlitePool>,
}

impl DatabaseRegistry {
  /// Create a new database registry
  pub fn new() -> Self {
    Self {
      pools: HashMap::new(),
    }
  }

  /// Add a database pool with a name
  pub fn register(&mut self, name: &str, pool: SqlitePool) {
    self.pools.insert(name.to_string(), pool);
  }

  /// Get a pool by name
  pub fn get(&self, name: &str) -> Option<&SqlitePool> {
    self.pools.get(name)
  }

  /// Get the default pool
  pub fn default(&self) -> Option<&SqlitePool> {
    self.pools.get("default")
  }
}
```

### Database Connection Enum

Create an enum to specify which database to use:

```rust
// src/database/database_registry.rs

/// Database identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatabaseName {
  Default,
  Analytics,
  Logging,
  // Add more as needed
}

impl DatabaseName {
  pub fn as_str(&self) -> &str {
    match self {
      DatabaseName::Default => "default",
      DatabaseName::Analytics => "analytics",
      DatabaseName::Logging => "logging",
    }
  }
}
```

Module wiring (so callers can `use crate::database::DatabaseRegistry;`):

```rust
// src/database/mod.rs

pub mod database;
pub mod database_registry;

pub use database::Database;
pub use database_registry::{DatabaseName, DatabaseRegistry};
```

### Updated Schema Building

**Before** (`src/dps_auth_api.rs:137-143`):
```rust
let database = self.initialize_database().await?;
let schema = crate::graphql::schema::build_schema()
  .data(database.pool)
  .data(self.config.as_ref().clone())
  .finish();
```

**After**:
```rust
let mut registry = DatabaseRegistry::new();
let default_db = self.initialize_database().await?;
registry.register("default", default_db.pool);

// Add second database (example)
let analytics_db = Database::new_with_pool_size(
  &self.config.sqlite_analytics_file_path,
  Some(self.config.sqlite_analytics_pool_size),
).await?;
registry.register("analytics", analytics_db.pool);

let schema = crate::graphql::schema::build_schema()
  .data(registry)
  .data(self.config.as_ref().clone())
  .finish();
```

### Updated Resolver Pattern

**Before**:
```rust
async fn add_site(&self, ctx: &Context<'_>, ...) -> Result<AddSiteResponse> {
  let pool = ctx.data::<SqlitePool>()?;
  let session_context = SessionContext::from_context(ctx)?;

  match AddSiteOrchestrator::run(pool, session_context.clone(), create_data).await {
    // ...
  }
}
```

**After** (Option A - Explicit Database Selection):
```rust
async fn add_site(&self, ctx: &Context<'_>, ...) -> Result<AddSiteResponse> {
  let registry = ctx.data::<DatabaseRegistry>()?;
  let pool = registry.default().ok_or(async_graphql::Error::new("Database not available"))?;
  let session_context = SessionContext::from_context(ctx)?;

  match AddSiteOrchestrator::run(pool, session_context.clone(), create_data).await {
    // ...
  }
}
```

**After** (Option B - Backward Compatible Default):
```rust
async fn add_site(&self, ctx: &Context<'_>, ...) -> Result<AddSiteResponse> {
  let pool = get_default_pool(ctx)?;
  let session_context = SessionContext::from_context(ctx)?;

  match AddSiteOrchestrator::run(pool, session_context.clone(), create_data).await {
    // ...
  }
}

// Helper function for backward compatibility
fn get_default_pool(ctx: &Context<'_>) -> Result<&SqlitePool, async_graphql::Error> {
  // Try new registry first
  if let Ok(registry) = ctx.data::<DatabaseRegistry>() {
    return registry
      .default()
      .ok_or_else(|| async_graphql::Error::new("Default database not available"));
  }
  // Fall back to old single pool for compatibility
  ctx.data::<SqlitePool>()
}
```

### Updated Orchestrator Pattern (When Multi-DB Needed)

**Before**:
```rust
pub async fn run(
  pool: &SqlitePool,
  session_context: SessionContext,
  create_data: CreateSiteData,
) -> Result<crate::models::Site, SiteError> {
  let mut conn = pool.acquire().await?;
  // ... single database logic
}
```

**After** (for orchestrators that need multiple databases):
```rust
pub async fn run(
  registry: &DatabaseRegistry,
  session_context: SessionContext,
  create_data: CreateSiteData,
) -> Result<crate::models::Site, SiteError> {
  let mut default_conn = registry
    .default()
    .ok_or(SiteError::DatabaseError("Default database not available".into()))?
    .acquire()
    .await?;

  let mut analytics_conn = registry
    .get("analytics")
    .ok_or(SiteError::DatabaseError("Analytics database not available".into()))?
    .acquire()
    .await?;

  // ... multi-database logic
}
```

**Note**: Orchestrators that only need the default database can keep using `&SqlitePool` parameter.

## Implementation Phases

### Phase 1: Create DatabaseRegistry Infrastructure
**Files to Modify**:
- `src/database/database_registry.rs` - New file containing `DatabaseRegistry` and `DatabaseName`
- `src/database/mod.rs` - Export the new module/types

**Implementation Details**:
1. Add `DatabaseRegistry` struct with `HashMap<String, SqlitePool>`
2. Add `DatabaseName` enum for type-safe database identifiers
3. Implement methods: `new()`, `register()`, `get()`, `default()`
4. Make `DatabaseRegistry` clone-safe (required for GraphQL context)

**Testing**:
1. Test registry creation and pool registration
2. Test pool retrieval by name
3. Test default pool behavior
4. Test thread-safety with concurrent access

### Phase 2: Update Application Initialization
**Files to Modify**:
- `src/dps_auth_api.rs` - Update `DpsAuthApiConfig` and `create_app()`

**Implementation Details**:
1. Add `sqlite_analytics_file_path: String` to `DpsAuthApiConfig`
2. Add `sqlite_analytics_pool_size: u16` to `DpsAuthApiConfig`
3. Update `create_app()` to create `DatabaseRegistry` instead of injecting pool directly
4. Register multiple databases into registry
5. Inject `DatabaseRegistry` into schema instead of `SqlitePool`

**Testing**:
1. Test application starts with single database (backward compatibility)
2. Test application starts with multiple databases
3. Test each database is accessible

### Phase 3: Add Helper for Backward Compatibility
**Files to Modify**:
- `src/graphql/resolvers/` - Add helper function (or create new module)

**Implementation Details**:
1. Create `get_default_pool(ctx: &Context<'_>)` helper function
2. Helper tries `DatabaseRegistry` first, falls back to `SqlitePool` for compatibility
3. Document that new resolvers should use `DatabaseRegistry` directly

**Testing**:
1. Test helper returns pool from registry
2. Test helper returns single pool when no registry (backward compatibility)
3. Test helper errors when neither is available

### Phase 4: Update Test Utilities
**Files to Modify**:
- `src/test_utils/mod.rs` - Update schema builders

**Implementation Details**:
1. Add `create_test_query_schema_with_registry()` function
2. Add `create_test_mutation_schema_with_registry()` function
3. Keep existing functions for backward compatibility
4. Add helper to create `DatabaseRegistry` with test pools

**Testing**:
1. Test existing tests still pass (no regression)
2. Test new functions work with registry
3. Test multi-database scenarios in tests

### Phase 5: Migration Strategy (Optional)
**Note**: This phase is optional and can be done incrementally

**Files to Modify**:
- `src/graphql/resolvers/*.rs` (migrate incrementally)

**Implementation Details**:
1. Migrate resolvers one at a time to use `DatabaseRegistry` directly
2. Use `get_default_pool()` helper during migration
3. Remove helper once all resolvers migrated
4. Update orchestrators that need multiple databases

**Testing**:
1. Run full test suite after each resolver migration
2. Verify no regressions

## Effort Estimation

### Phase 1: DatabaseRegistry Infrastructure
- **Development**: 2-3 hours
- **Testing**: 1-2 hours
- **Total**: 3-5 hours

### Phase 2: Application Initialization Updates
- **Development**: 2-3 hours
- **Testing**: 1-2 hours
- **Total**: 3-5 hours

### Phase 3: Backward Compatibility Helper
- **Development**: 1 hour
- **Testing**: 1 hour
- **Total**: 2 hours

### Phase 4: Test Utilities Update
- **Development**: 1-2 hours
- **Testing**: 1-2 hours
- **Total**: 2-4 hours

### Phase 5: Incremental Migration (Optional)
- **Development**: 15-20 resolvers × 30 min each = 8-10 hours
- **Testing**: Run full test suite after each batch
- **Total**: 8-10 hours

**Total Effort** (excluding optional Phase 5): **10-16 hours**
**Total Effort** (including optional Phase 5): **18-26 hours**

## Advantages of This Approach

1. **Thread Safety**: `DatabaseRegistry` is `Clone`, pools are thread-safe
2. **Backward Compatibility**: Helper function allows gradual migration
3. **Type Safety**: `DatabaseName` enum prevents string typos
4. **Flexibility**: Easy to add new databases without changing resolver signatures
5. **Performance**: No performance overhead (registry is a HashMap lookup)
6. **Async Compatibility**: All pools remain async-compatible

## Considerations and Trade-offs

### Memory Usage
- **Current**: Single pool overhead
- **New**: Registry + multiple pools overhead
- **Impact**: Minimal (a few MB per additional pool)

### Connection Limits
- **Current**: Single pool with configured size
- **New**: Each pool has its own configured size
- **Consideration**: May need to tune individual pool sizes based on database usage patterns

### Error Handling
- **Current**: Simple `ctx.data::<SqlitePool>()?`
- **New**: Need to handle "database not found" errors
- **Mitigation**: Provide `.unwrap_or()` pattern for default database

### Code Complexity
- **Current**: Simple and direct
- **New**: Additional abstraction layer
- **Mitigation**: Backward compatibility helper reduces initial complexity

## Existing Crates That Could Help

### 1. `sqlx` (already in use)
- **Pros**: Already integrated, thread-safe pools, well-tested
- **Cons**: No built-in multi-database registry (need custom implementation)
- **Recommendation**: Continue using sqlx pools in custom registry

### 2. `deadpool-sqlx`
- **Pros**: More advanced pool management, better metrics
- **Cons**: Another dependency, sqlx pools are sufficient for current needs
- **Recommendation**: Not necessary - sqlx pools work well

### 3. `r2d2` (connection pool)
- **Pros**: Mature, widely used
- **Cons**: Not async-native, sqlx pools are better for async
- **Recommendation**: Do not use - sqlx async pools are better

**Recommendation**: Continue using sqlx's `SqlitePool` with custom `DatabaseRegistry`. No additional crates needed.

## Conclusion

The proposed `DatabaseRegistry` approach provides a clean, thread-safe way to support multiple databases while maintaining backward compatibility. The implementation is straightforward and doesn't require external dependencies beyond existing sqlx integration.

Key benefits:
- All databases accessible to all resolvers
- Thread-safe and async-compatible
- Minimal code changes (with backward compatibility helper)
- Flexible for future database additions

The effort estimate of 10-16 hours (excluding optional migration) represents a reasonable investment for the flexibility gained.
