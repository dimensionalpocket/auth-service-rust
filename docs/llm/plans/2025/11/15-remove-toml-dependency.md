# Plan: Remove unused toml dependency
Date: 2025-11-15@22:37

Summary:
- Remove direct dependency `toml` from [`Cargo.toml`](Cargo.toml:33) if verification shows it is unused and all builds/tests pass.

Context:
- Automated check reported `toml` as unused (see [`docs/llm/plans/2025/11/15-analyze-unused-deps.md`](docs/llm/plans/2025/11/15-analyze-unused-deps.md:9)).

Goals:
- Safely remove the direct `toml` dependency.
- Verify builds, tests, and binaries still work.
- Leave `toml` only as a transitive dependency if required.

Preconditions:
- Local environment can run mise/wrapper: use mise exec -- for cargo commands.

Steps:
1. Verify current state (automated scan + full build)
   - Run:
```bash
mise exec -- cargo +nightly udeps --all-targets --all-features --manifest-path Cargo.toml
```
   - Then:
```bash
mise exec -- cargo build --all-features --manifest-path Cargo.toml
mise exec -- cargo test --all-features --manifest-path Cargo.toml
```

2. Grep for explicit usage
   - Search source and scripts:
```bash
rg "use toml::|toml::|toml\s*=" src/ scripts/ config/ || true
```

3. If verification shows `toml` unused, remove it from [`Cargo.toml`](Cargo.toml:33)
   - Edit file and remove the line:
```toml
toml = "0.8"
```

4. Rebuild and re-run tests (same commands as step 1)

5. Confirm `toml` remains only transitively
```bash
mise exec -- cargo tree --manifest-path Cargo.toml | rg "toml" || true
```

6. Inspect special places that can hide usage:
   - build scripts, proc-macros, binaries, and doc-tests:
     - [`scripts/run_local_server.rs`](scripts/run_local_server.rs:1)
     - [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1)

7. Update documentation
   - Update [`docs/llm/plans/2025/11/15-analyze-unused-deps.md`](docs/llm/plans/2025/11/15-analyze-unused-deps.md:18) with commands run and final decision.

Rollback / investigation plan if failures:
- If build/tests fail after removal:
  - Run failing build/test to see error.
  - Inspect error for location (doc-tests, build.rs, binaries).
  - If quick fix exists (change code to avoid toml), propose patches.
  - Otherwise restore the `toml` line and document reason.

Files to modify (proposed):
- [`Cargo.toml`](Cargo.toml:33)
- [`docs/llm/plans/2025/11/15-analyze-unused-deps.md`](docs/llm/plans/2025/11/15-analyze-unused-deps.md:18)

Commands summary (copy-paste)
```bash
mise exec -- cargo +nightly udeps --all-targets --all-features --manifest-path Cargo.toml
mise exec -- cargo build --all-features --manifest-path Cargo.toml
mise exec -- cargo test --all-features --manifest-path Cargo.toml
mise exec -- cargo tree --manifest-path Cargo.toml | rg "toml" || true
rg "use toml::|toml::|toml\s*=" src/ scripts/ config/ || true
```

Estimated time:
- Verification: 5–15 minutes (depends on test suite)
- Removal + re-check: 5–10 minutes
- If failures: additional time depending on fix complexity

Approval:
- Proceed to implement this plan after user approval. Implementation will modify [`Cargo.toml`](Cargo.toml:33) and update the plan file with results.