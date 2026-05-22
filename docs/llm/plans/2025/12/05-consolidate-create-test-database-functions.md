# Consolidate create_test_database() Functions

## Analysis

### Current Situation
There are currently **two** `create_test_database()` functions defined in the codebase:

1. **Primary function**: `src/database/mod.rs:360` in the `test_utils` module
2. **Duplicate function**: `tests/database_integration_tests.rs:13` 

### Primary Function (src/database/mod.rs:360)
```rust
pub async fn create_test_database() -> (SqlitePool, NamedTempFile) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();
    let database = Database::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");
    let pool = database.pool;

    // Configure SQLite settings (same as production)
    Database::configure_sqlite(&pool)
      .await
      .expect("Failed to configure SQLite");

    // Run migrations
    let database = Database { pool: pool.clone() };
    database.migrate().await.expect("Failed to run migrations");

    (pool, temp_file)
}
```

### Duplicate Function (tests/database_integration_tests.rs:13)
```rust
async fn create_test_database() -> (SqlitePool, NamedTempFile) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let sqlite_file_path = temp_file.path().display().to_string();

    let database = Database::new(&sqlite_file_path)
      .await
      .expect("Failed to create test database");

    // Run migrations
    database.migrate().await.expect("Failed to run migrations");

    (database.pool, temp_file)
}
```

### Key Differences
1. **SQLite Configuration**: Primary function calls `Database::configure_sqlite()` while duplicate does not
2. **Database Instance**: Primary function recreates Database instance for migrations, duplicate uses existing
3. **Visibility**: Primary is `pub`, duplicate is private
4. **Module**: Primary is in `test_utils` module, duplicate is standalone

### Usage Analysis
- **Primary function**: Used by 35+ files across services, orchestrators, resolvers, and queries
- **Duplicate function**: Used only within `tests/database_integration_tests.rs`
- **Integration tests**: Use `create_app()` function which creates full app with embedded database

## Consolidation Plan

### Phase 1: Enhance Primary Function
**File**: `src/database/mod.rs:360`

1. **Add optional configuration parameter**:
   ```rust
   pub async fn create_test_database() -> (SqlitePool, NamedTempFile) {
       create_test_database_with_config(true).await
   }

   pub async fn create_test_database_with_config(configure_sqlite: bool) -> (SqlitePool, NamedTempFile) {
       let temp_file = NamedTempFile::new().expect("Failed to create temp file");
       let sqlite_file_path = temp_file.path().display().to_string();
       let database = Database::new(&sqlite_file_path)
         .await
         .expect("Failed to create test database");
       let pool = database.pool;

       // Optionally configure SQLite settings
       if configure_sqlite {
           Database::configure_sqlite(&pool)
             .await
             .expect("Failed to configure SQLite");
       }

       // Run migrations
       let database = Database { pool: pool.clone() };
       database.migrate().await.expect("Failed to run migrations");

       (pool, temp_file)
   }
   ```

### Phase 2: Update Duplicate Function Usage
**File**: `tests/database_integration_tests.rs`

1. **Remove duplicate function**
2. **Add import for primary function**:
   ```rust
   use dps_auth_api::database::test_utils::create_test_database;
   ```
3. **Update test usage** to use primary function (no changes needed as signature is identical)

### Phase 3: Verify Integration Tests
**File**: `tests/integration_tests.rs`

1. **Confirm integration tests use `create_app()` function** (not `create_test_database()`)
2. **No changes needed** as integration tests already use proper full-app approach

### Phase 4: Testing
1. **Run unit tests**: `cargo test --quiet`
2. **Run integration tests**: `cargo test integration_tests --verbose`
3. **Verify all tests pass** with consolidated function

## Benefits of Consolidation

1. **Single Source of Truth**: One canonical implementation
2. **Consistent SQLite Configuration**: All tests get production-like settings
3. **Reduced Code Duplication**: Eliminates 12 lines of duplicate code
4. **Easier Maintenance**: Changes only need to be made in one place
5. **Better Test Reliability**: Consistent database setup across all unit tests

## Files to Modify

1. `src/database/mod.rs` - Enhance primary function with optional configuration
2. `tests/database_integration_tests.rs` - Remove duplicate function and add import

## Files to Verify (No Changes Expected)

1. `tests/integration_tests.rs` - Should continue using `create_app()` function
2. All 35+ files using primary function - Should continue working unchanged

## Risk Assessment

**Low Risk**: 
- Primary function is more robust (includes SQLite configuration)
- Signature remains identical for existing users
- Integration tests are unaffected
- Easy rollback if issues arise

## Success Criteria

1. All unit tests pass with consolidated function
2. All integration tests continue to pass
3. No duplicate `create_test_database()` functions remain
4. Code coverage remains the same