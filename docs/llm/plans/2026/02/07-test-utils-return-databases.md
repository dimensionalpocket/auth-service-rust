# Test Utils: Return Databases From create_test_database*

## Date
2026-02-07

## Goal
Update the `create_test_database*` helpers in `src/test_utils/mod.rs` so they return a `Databases` instance (instead of a `SqlitePool`). Call sites throughout `src/` and `tests/` will then obtain the main pool via `databases.main()`.

This plan is intentionally scoped to test utilities + replacements only (no Phase 2 app initialization work).

## Non-Goals
- Do not change production app schema injection yet (`src/dps_auth_api.rs`).
- Do not update resolvers/orchestrators to accept `Databases` yet.
- Do not introduce new crates.

## Current State
- `Databases` exists at `src/database/databases.rs` and is exported by `src/database/mod.rs`.
- `src/test_utils/mod.rs` currently exposes:
  - `create_test_database() -> (SqlitePool, NamedTempFile)`
  - `create_test_database_with_pool_size(pool_size: u32) -> (SqlitePool, NamedTempFile)`
  - `create_test_database_with_config(configure_sqlite: bool) -> (SqlitePool, NamedTempFile)`
  - `create_test_database_with_config_and_pool_size(configure_sqlite: bool, pool_size: u32) -> (SqlitePool, NamedTempFile)`
- There are many call sites that pattern-match the tuple as `(pool, _tmp)` or similar.

## Design Decisions
1. Keep the helper names stable (`create_test_database*`) to minimize churn; only change return types.
2. `Databases` will be constructed with:
   - `main`: a migrated, optionally configured SQLite DB (same behavior as today)
   - `session`: a separate SQLite DB (created/configured) with no migrations/seeds for now
3. Preserve the temp file lifetime guarantee:
   - Return both tempfiles so the DB files remain alive for the duration of the test.

## API Changes (Test Utils)
Update signatures:

```rust
pub async fn create_test_database() -> (Databases, NamedTempFile, NamedTempFile);
pub async fn create_test_database_with_pool_size(pool_size: u32) -> (Databases, NamedTempFile, NamedTempFile);
pub async fn create_test_database_with_config(configure_sqlite: bool) -> (Databases, NamedTempFile, NamedTempFile);
pub async fn create_test_database_with_config_and_pool_size(
  configure_sqlite: bool,
  pool_size: u32,
) -> (Databases, NamedTempFile, NamedTempFile);
```

Notes:
- The ordering will be `(databases, main_tmp, session_tmp)`.
- Existing helper functions that accept `pool: &SqlitePool` remain unchanged; call sites will pass `databases.main()`.

## Implementation Steps

### Phase A: Update test utils implementation
- Modify `src/test_utils/mod.rs`:
  - Import `crate::database::{Databases, MainDatabase, SessionDatabase}`.
  - In `create_test_database_with_config_and_pool_size`:
    - Create `main_tmp` and `session_tmp`.
    - Build `MainDatabase` from `main_tmp` path with pool size.
    - Optionally call `MainDatabase::configure_sqlite(&main_pool)` (same as today).
    - Run main migrations (same as today).
    - Build `SessionDatabase` from `session_tmp` path (pool size can default to 1; keep it simple).
    - Construct `Databases::new(main_pool, session_pool)`.
    - Return `(databases, main_tmp, session_tmp)`.
  - Update the internal tests in `src/test_utils/mod.rs` to use `databases.main()`.

### Phase B: Systematic call-site replacements (src/ + tests/)
The main mechanical change:

Before:
```rust
let (pool, _tmp) = create_test_database().await;
// ... use pool
```

After:
```rust
let (databases, _main_tmp, _session_tmp) = create_test_database().await;
let pool = databases.main();
// ... use pool
```

Also handle variants:
- `create_test_database_with_pool_size(...)`
- `create_test_database_with_config(...)`
- `create_test_database_with_config_and_pool_size(...)`

Key patterns to look for:
- Tuple destructuring:
  - `let (pool, tmp) = ...;`
  - `let (pool, _temp_file) = ...;`
  - `let (pool, _tmp) = ...;`
  - `let (pool, temp_file) = ...;` (temp used; must update usage)
- Passing the pool into schema helpers:
  - `create_test_query_schema(..., Some(pool), ...)`
  - `create_test_mutation_schema(..., Some(pool), ...)`
- Pool clones:
  - `Some(pool.clone())` becomes `Some(databases.main().clone())` (or bind `let pool = databases.main().clone();`).

### Phase C: Build a one-off replacement script
Goal: reduce manual edits and handle the bulk change safely.

Constraints:
- Keep it repo-local; no new dependencies.

Recommended approach:
1. Use `rg` to collect all matching lines + file paths.
2. Use a small Bun script under `scripts/` to apply targeted replacements (preferred for speed of iteration). Fall back to Rust or Python if needed.

Script responsibilities:
- Parse files and apply conservative transformations for the common patterns:
  1) Replace `let (pool, <tmp>) = create_test_database...await;` with:
     - `let (databases, <main_tmp>, <session_tmp>) = create_test_database...await;`
     - Add `let pool = databases.main();` on the next line.
     - If `<tmp>` is a named variable and used later, map it to `<main_tmp>` (since that is the previous behavior).
  2) Update `Some(pool)` / `Some(pool.clone())` occurrences in the same scope when `pool` is now a reference.

Fallback:
- Any file that doesn’t match the “simple tuple destructure” pattern should be flagged by the script for manual follow-up.

Validation after script:
- `cargo test --quiet` to catch missed edge cases.

### Phase D: Fix compilation errors iteratively
Expected error categories:
- Mismatched tuple arity in destructuring.
- `pool` now being a `&SqlitePool` reference; places that require owned `SqlitePool` need `.clone()`.
- Tests that only use a pool for `create_test_query_schema` will need `Some(databases.main().clone())`.
- Any test that relied on the single tempfile returned may need updating if it used the path.

## Testing
1. Unit tests in `src/test_utils/mod.rs` should be updated and continue to pass.
2. Run full suite:
   - `cargo test --quiet`
3. Spot-check a few GraphQL resolver tests that build schemas with context data, ensuring the change is only in pool creation, not schema injection.

## Anticipated Challenges / Edge Cases
- Some call sites may have:
  - `let (pool, tmp) = ...;` and then use `tmp.path()`; we must preserve that behavior by returning `main_tmp` and using it in place of `tmp`.
- Some call sites may use `pool` by value in a way that relied on ownership.
  - Fix by converting to `databases.main().clone()` where an owned `SqlitePool` is required.
- Some call sites may call `create_test_database*` inside helper functions or closures, making automated replacements harder.
- A few tests might not want/need the session DB; overhead is minimal but if it becomes noticeable, consider lazy session creation later (out of scope here).

## Files Expected To Change
- `src/test_utils/mod.rs`
- Many files under:
  - `src/**` (unit tests embedded in modules)
  - `tests/**` (integration tests)
- Optional helper script:
  - `scripts/replace_test_db_helpers.ts` (Bun) (or `.py` / `.rs`)
