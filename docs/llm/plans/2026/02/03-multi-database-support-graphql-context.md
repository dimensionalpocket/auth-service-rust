# Multi-Database Support for GraphQL Context

## Date
2026-02-03

## Summary
Analysis of current database pool usage in GraphQL contexts and plan for supporting multiple SQLite database instances. All database instances should be accessible to all resolvers in a multi-threaded/async scenario.

## Current State Analysis

### Database Module Layout

- Each database is its own struct so it can own its:
  - SQLite PRAGMA command list
  - migration path (`config/databases/<name>/migrations`)
  - seed path (`config/databases/<name>/seeds`)
- Today, the primary database struct is `MainDatabase` in `src/database/main_database.rs`.
- `src/database/mod.rs` re-exports database structs (currently `MainDatabase`).

### Database Pool Injection Flow

1. **Database Initialization** (`src/database/main_database.rs`):
   - `MainDatabase` struct contains a single `pub pool: SqlitePool`
   - Pool is created via `MainDatabase::new()` or `MainDatabase::new_with_pool_size()`
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

1. **Resolver Retrieval** (current state):
   ```rust
   let pool = ctx.data::<SqlitePool>()?;
   ```
   Used in:
   - `src/graphql/resolvers/add_site.rs:76`
   - `src/graphql/resolvers/users.rs:49`
   - `src/graphql/resolvers/auth_login.rs:35`
   - And all other resolvers that need database access

2. **Orchestrator Passing** (all orchestrators, current state):
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

Planned change: resolvers will stop extracting pools and instead pass `&Databases` into orchestrators. Orchestrators will be responsible for selecting pools and acquiring connections.

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

### Database Trait (Reusable Core)

Add a `Database` trait in `src/types/database.rs` implemented by all database structs (`MainDatabase`, `SessionDatabase`). This keeps each database in its own file (so it can own paths + PRAGMA list) while extracting shared behavior.

Suggested shape:

```rust
pub trait Database {
  fn pool(&self) -> &SqlitePool;

  fn migrations_dir() -> &'static str;
  fn seeds_dir() -> &'static str;

  fn pragma_commands() -> &'static [&'static str];

  async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError>;
  async fn seed(&self) -> Result<(), Box<dyn std::error::Error>>;
  async fn dump_schema_content(&self) -> Result<String, sqlx::Error>;
  async fn dump_schema_to_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>>;
}
```

Implementation approach (recommended): keep the heavy implementation in a shared internal helper module (e.g. `src/database/sqlite_database.rs`) so each `*Database` struct only defines constants/paths + PRAGMA list.

Reuse suggestion (concrete):

```rust
pub trait DatabaseSpec {
  const MIGRATIONS_DIR: &'static str;
  const SEEDS_DIR: &'static str;
  const PRAGMA_COMMANDS: &'static [&'static str];
}

pub struct SqliteDatabaseCore<S: DatabaseSpec> {
  pub pool: SqlitePool,
  _marker: std::marker::PhantomData<S>,
}

// Implement new_with_pool_size/configure/migrate/seed/dump once here using S::* constants.
```

Then `MainDatabase` and `SessionDatabase` become thin wrappers around the shared core.

### Requirements
1. Support multiple SQLite database instances (main + session)
2. All databases accessible to all resolvers
3. Maintain thread safety and async compatibility
4. Breaking change is acceptable; update resolvers/orchestrators/tests accordingly
5. Each database is represented by a dedicated struct (`MainDatabase`, `SessionDatabase`) so each can keep its own PRAGMA/migration/seed behavior

### Proposed Solution: Databases

Create a `Databases` struct with one field per database (no dynamic registration). The pools are produced by dedicated database structs (e.g., `MainDatabase`, `SessionDatabase`).

```rust
// src/database/databases.rs

/// Application database pools
#[derive(Clone)]
pub struct Databases {
  main: SqlitePool,
  session: SqlitePool,
}

impl Databases {
  pub fn new(main: SqlitePool, session: SqlitePool) -> Self {
    Self {
      main,
      session,
    }
  }

  pub fn main(&self) -> &SqlitePool {
    &self.main
  }

  pub fn session(&self) -> &SqlitePool {
    &self.session
  }

}
```

Module wiring (so callers can `use crate::database::Databases;`):

```rust
// src/database/mod.rs

pub mod main_database;
pub mod databases;

pub use main_database::MainDatabase;
pub use databases::Databases;
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
let main_db = self.initialize_database().await?;

let session_db = SessionDatabase::new_with_pool_size(
  &self.config.sqlite_session_file_path,
  Some(self.config.sqlite_session_pool_size),
)
.await?;

let databases = Databases::new(main_db.pool, session_db.pool);

let schema = crate::graphql::schema::build_schema()
  .data(databases)
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

**After** (Resolvers pass databases to orchestrators):
```rust
async fn add_site(&self, ctx: &Context<'_>, ...) -> Result<AddSiteResponse> {
  let databases = ctx.data::<Databases>()?;
  let session_context = SessionContext::from_context(ctx)?;

  match AddSiteOrchestrator::run(databases, session_context.clone(), create_data).await {
    // ...
  }
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

**After** (orchestrators extract pools from databases):
```rust
pub async fn run(
  databases: &Databases,
  session_context: SessionContext,
  create_data: CreateSiteData,
) -> Result<crate::models::Site, SiteError> {
  let mut main_conn = databases.main().acquire().await?;

  // Optionally, also use another database in the same workflow
  let mut session_conn = databases.session().acquire().await?;

  // ... multi-database logic
}
```

**Note**: With this approach, orchestrators are the single place that decides which pool(s) to use.

## Implementation Phases

### Phase 0: Create SessionDatabase + Shared Trait
**Files to Create**:
- `src/database/session_database.rs`
- `src/types/database.rs` - `Database` trait
 - (recommended) `src/database/sqlite_database.rs` - shared implementation used by all database structs

**Folders/Files to Create** (empty placeholders):
- `config/databases/session/migrations/.keep`
- `config/databases/session/seeds/.keep`

**Files to Modify**:
 - `src/database/mod.rs` - export `SessionDatabase`
- `src/types/mod.rs` - export the `Database` trait

**Implementation Details**:
1. Create `Database` trait based on existing `MainDatabase` behavior (pool access, PRAGMA config hook, migrate/seed/schema dump operations)
2. Implement the trait for `MainDatabase`
3. Implement `SessionDatabase` mirroring `MainDatabase` but with:
   - distinct migration dir (`config/databases/session/migrations`)
   - distinct seed dir (`config/databases/session/seeds`)
   - distinct schema dump headers/paths if needed
   - distinct PRAGMA command lists (initially may match `MainDatabase`, but kept separate)
4. Recommended reuse: extract shared SQLite bootstrap/migrate/seed/dump code into a shared helper module so the two structs stay thin and only define constants/paths/PRAGMAs

**Testing**:
1. Unit test that `SessionDatabase` can be created (tempfile path), configured, and used for schema dump (even if empty)
2. Unit test that `migrate()` on session with empty migrations dir is a no-op/succeeds (depending on sqlx behavior)
3. Unit test that `seed()` on session with empty seeds dir is a no-op/succeeds

### Phase 1: Create Databases Infrastructure
**Files to Modify**:
- `src/database/databases.rs` - New file containing `Databases`
- `src/database/mod.rs` - Export the new module/types

**Implementation Details**:
1. Add `Databases` struct with one field per database (`main`, `session`)
2. Implement constructor `new(main, session)`
3. Implement explicit accessors: `main()`, `session()`
4. Make `Databases` clone-safe (required for GraphQL context)

**Testing**:
1. Test `Databases` creation
2. Test pool accessors (`main()`, `session()`)
3. Test main pool behavior
4. Test thread-safety with concurrent access

### Phase 2: Update Application Initialization
**Files to Modify**:
- `src/dps_auth_api.rs` - Update `DpsAuthApiConfig` and `create_app()`

**Implementation Details**:
1. Add `sqlite_session_file_path: String` to `DpsAuthApiConfig`
2. Add `sqlite_session_pool_size: u16` to `DpsAuthApiConfig`
3. Construct `Databases` and inject it into the schema instead of `SqlitePool`

**Testing**:
1. Test application starts with all configured databases
2. Test each database is accessible via `Databases`

### Phase 3: Update Orchestrators to Use Databases
**Files to Modify**:
- `src/orchestrators/**` - Update orchestrator signatures and pool selection

**Implementation Details**:
1. Change orchestrator `run(...)` signatures from `pool: &SqlitePool` to `databases: &Databases`
2. Inside each orchestrator, extract the required pool(s) from `databases` (`main()`, `session()`)
3. Acquire one connection per required pool and pass `&mut SqliteConnection` down into services/queries as today
4. Keep resolvers thin: no pool selection or connection acquisition in resolvers

**Testing**:
1. Update unit tests (if any) that call orchestrators directly
2. Run full test suite

### Phase 4: Update Resolvers to Pass Databases
**Files to Modify**:
- `src/graphql/resolvers/*.rs`

**Implementation Details**:
1. Replace `ctx.data::<SqlitePool>()?` with `ctx.data::<Databases>()?`
2. Pass `&Databases` into orchestrator `run(...)`
3. Leave all database selection to orchestrators

**Testing**:
1. Run resolver tests (unit/integration) to ensure schema has `Databases` injected

### Phase 5: Update Test Utilities
**Files to Modify**:
- `src/test_utils/mod.rs` - Update schema builders

**Implementation Details**:
1. Add `create_test_query_schema_with_databases()` function
2. Add `create_test_mutation_schema_with_databases()` function
3. Update/replace existing schema helpers to inject `Databases` (breaking change)
4. Add helper to create `Databases` with test pools

**Testing**:
1. Update tests to use databases-injecting schema builders
2. Test schema builders work with databases
3. Test multi-database scenarios in tests

### Phase 6: Incremental Multi-DB Adoption (Optional)
**Note**: This phase is optional and can be done incrementally

**Files to Modify**:
- `src/orchestrators/**` (only workflows that should use multiple databases)

**Implementation Details**:
1. Start with all orchestrators using `databases.main()` only
2. For workflows that need additional databases, use `databases.session()` pool accessor
3. Keep the rest of the stack unchanged: services/queries still operate on a connection

**Testing**:
1. Add tests for any new cross-database workflows
2. Run full test suite after each batch

## Effort Estimation

### Phase 0: Create SessionDatabase + Shared Trait
- **Development**: 3-6 hours
- **Testing**: 2-4 hours
- **Total**: 5-10 hours

### Phase 1: Databases Infrastructure
- **Development**: 2-3 hours
- **Testing**: 1-2 hours
- **Total**: 3-5 hours

### Phase 2: Application Initialization Updates
- **Development**: 2-3 hours
- **Testing**: 1-2 hours
- **Total**: 3-5 hours

### Phase 3: Update Orchestrators to Use Databases
- **Development**: 2-4 hours
- **Testing**: 1-2 hours
- **Total**: 3-6 hours

### Phase 4: Update Resolvers to Pass Databases
- **Development**: 1-2 hours
- **Testing**: 1-2 hours
- **Total**: 2-4 hours

### Phase 5: Test Utilities Update
- **Development**: 1-2 hours
- **Testing**: 1-2 hours
- **Total**: 2-4 hours

### Phase 6: Incremental Multi-DB Adoption (Optional)
- **Development**: Varies by workflow count/complexity
- **Testing**: Add/extend tests per workflow

**Total Effort** (excluding optional Phase 6): **15-29 hours**
**Total Effort** (including optional Phase 6): **TBD**

## Advantages of This Approach

1. **Thread Safety**: `Databases` is `Clone`, pools are thread-safe
2. **Single Responsibility**: Orchestrators own pool selection and connection acquisition
3. **Type Safety**: No string keys; accessors are compile-time checked
4. **Flexibility**: Easy to add new databases without changing resolver signatures
5. **Performance**: No lookup overhead (direct field access)
6. **Async Compatibility**: All pools remain async-compatible

## Considerations and Trade-offs

### Memory Usage
- **Current**: Single pool overhead
 - **New**: Multiple pools overhead
- **Impact**: Minimal (a few MB per additional pool)

### Connection Limits
- **Current**: Single pool with configured size
- **New**: Each pool has its own configured size
- **Consideration**: May need to tune individual pool sizes based on database usage patterns

### Error Handling
- **Current**: Simple `ctx.data::<SqlitePool>()?`
- **New**: Errors are primarily connection acquisition / query execution errors (not missing database selection)
- **Mitigation**: Keep errors explicit at the orchestrator boundary (e.g., when session operations fail)

### Code Complexity
- **Current**: Simple and direct
- **New**: Additional abstraction layer
- **Mitigation**: Keep the API surface small: resolvers only pass `&Databases`; orchestrators handle pool selection

## Existing Crates That Could Help

### 1. `sqlx` (already in use)
- **Pros**: Already integrated, thread-safe pools, well-tested
- **Cons**: No built-in multi-database container type (need custom implementation)
- **Recommendation**: Continue using sqlx pools with custom `Databases`

### 2. `deadpool-sqlx`
- **Pros**: More advanced pool management, better metrics
- **Cons**: Another dependency, sqlx pools are sufficient for current needs
- **Recommendation**: Not necessary - sqlx pools work well

### 3. `r2d2` (connection pool)
- **Pros**: Mature, widely used
- **Cons**: Not async-native, sqlx pools are better for async
- **Recommendation**: Do not use - sqlx async pools are better

**Recommendation**: Continue using sqlx's `SqlitePool` with custom `Databases`. No additional crates needed.

## Conclusion

The proposed `Databases` approach provides a clean, thread-safe way to support multiple databases. This is a breaking change: the GraphQL schema context will carry `Databases` (not `SqlitePool`), resolvers will pass the databases object, and orchestrators will select pools.

Key benefits:
- All databases accessible to all resolvers
- Thread-safe and async-compatible
- Clear responsibility split (resolver: pass inputs; orchestrator: choose DB/pool; service/query: use connections)
- Flexible for future database additions

The effort estimate of 10-16 hours (excluding optional migration) represents a reasonable investment for the flexibility gained.
