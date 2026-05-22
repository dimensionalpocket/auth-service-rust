# 2025-11-15 analyze unused deps

## Context
- Followed project instructions in [`docs/llm/instructions.md`](docs/llm/instructions.md:1).
- Performed code search and ran cargo-udeps with the required mise wrapper.

## Findings
- Directly used crates (examples): [`axum`](Cargo.toml:10), [`async-graphql`](Cargo.toml:8), [`sqlx`](Cargo.toml:25), [`dps-auth-session`](Cargo.toml:30), [`dps-config`](Cargo.toml:31), [`argon2`](Cargo.toml:22), [`bcrypt`](Cargo.toml:23), [`tracing`](Cargo.toml:17), [`uuid`](Cargo.toml:19), [`tempfile`](Cargo.toml:27), [`regex`](Cargo.toml:20).
- Automated check (mise exec -- cargo +nightly udeps) reported one unused direct dependency:
  - `toml` declared in [`Cargo.toml`](Cargo.toml:33)

## Evidence
- udeps output: "unused dependencies: `toml`" (run performed in workspace).
- No `use toml::` or direct `toml` references found in `src/` files; examples inspected: [`src/dps_auth_api.rs`](src/dps_auth_api.rs:77), [`src/middleware/session.rs`](src/middleware/session.rs:1), [`src/graphql/schema.rs`](src/graphql/schema.rs:75).
- `toml` still appears transitively in `Cargo.lock` via tooling/proc-macro deps; this is expected and acceptable.

## Recommended next steps (manual verification)
1) Verify locally with:
   - mise exec -- cargo +nightly udeps --all-targets --all-features --manifest-path Cargo.toml
   - mise exec -- cargo build --all-features --manifest-path Cargo.toml
   - mise exec -- cargo test --all-features --manifest-path Cargo.toml
   - mise exec -- cargo tree --manifest-path Cargo.toml | rg "toml" || true
2) If build/test pass with `toml` removed from [`Cargo.toml`](Cargo.toml:33), remove the dependency.
3) If any failures occur, investigate doc-tests, build scripts, or binaries (see [`scripts/run_local_server.rs`](scripts/run_local_server.rs:1), [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1)) for `toml` usage.

## Stop
- No implementation is performed in this plan. Awaiting user approval to proceed.