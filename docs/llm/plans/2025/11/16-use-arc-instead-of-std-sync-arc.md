# Use Arc instead of std::sync::Arc (revised)

Date: 2025-11-16

Summary
- Replace fully-qualified [`std::sync::Arc;`](docs/llm/plans/2025/11/16-use-arc-instead-of-std-sync-arc.md:1) usages in source files with a short, local import and plain [`Arc<T>`](docs/llm/plans/2025/11/16-use-arc-instead-of-std-sync-arc.md:1) for readability.
- Keep imports local to the module to avoid surprising re-exports or ambiguity.

Rationale
- `use std::sync::Arc;` shortens type signatures and improves scan-ability.
- The underlying type is identical; this is purely a stylistic/clarity change.
- Avoid global re-exports; prefer adding `use` near other imports in each file.

Files to inspect (priority)
- [`src/middleware/session.rs:1`](src/middleware/session.rs:1)
- [`src/graphql/resolvers/create_session.rs:1`](src/graphql/resolvers/create_session.rs:1)
- [`src/dps_auth_api.rs:1`](src/dps_auth_api.rs:1)
- Additional `*.rs` files found by a repo-wide search.

Before / after (example)
- Clickable shorthand of language construct: [`use std::sync::Arc;`](docs/llm/plans/2025/11/16-use-arc-instead-of-std-sync-arc.md:1)

Before (explicit)
```rust
// rust
fn new_session(svc: std::sync::Arc<crate::services::SessionService>) -> Handler {
    // ...
}
```

After (preferred)
```rust
// rust
use std::sync::Arc;

fn new_session(svc: Arc<crate::services::SessionService>) -> Handler {
    // ...
}
```

Replacement strategy
1. Run a tolerant regex search for `std::sync::Arc` variants across repository.
2. For each file with matches:
   - Add `use std::sync::Arc;` near the top with other `use` lines.
   - Replace occurrences of `std::sync::Arc<...>` with `Arc<...>` only in that file.
   - If the file already imports another `Arc` (from a crate), skip and leave the fully-qualified path or use an explicit alias (rare).
3. Apply targeted edits per file (small patches), not a bulk replace across the repo.
4. Build and test after each group of edits.

Ambiguities & mitigations
- If a file already imports a different `Arc`, do not blindly replace — keep `std::sync::Arc` or alias:
```rust
use std::sync::Arc as StdArc;
```
- For public APIs: the visible type name remains `Arc<T>`; adding a local `use` doesn't change the API.

Testing & verification
- Build and test with the project environment: `mise exec -- cargo build` and `mise exec -- cargo test`.
- Fix any compile errors that surface due to shadowing or missing imports.

Plan of work (actionable steps)
- [ ] Run repo search for `std::sync::Arc` and gather file list.
- [ ] Prepare per-file diffs: add `use std::sync::Arc;` and replace occurrences.
- [ ] Apply diffs in small batches, run `mise exec -- cargo test` after each batch.
- [ ] Address any ambiguous cases with explicit aliases and document choices.
- [ ] Update this plan with files modified and test outcomes.

Deliverables
- A short summary of files changed and the exact edits.
- Patch files / diffs for review (no git operations performed).
- Tests passing after edits.

Approval request
- Confirm if you want me to:
  - Apply the edits automatically in the repo (targeted changes, run tests), or
  - Produce a patch proposal (diffs) for manual review and apply.