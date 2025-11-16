# Plan: Replace crate:: fully-qualified paths with local imports for readability

Context
- I've read [`docs/llm/instructions.md`](docs/llm/instructions.md:1) and project README.

Goal
- Replace occurrences of fully-qualified paths using crate:: in code with local imports to improve readability (no behavior change).

Analysis
- Occurrences are namespace-qualified type paths (examples: `Arc<crate::DpsAuthApiConfig>`, `crate::DpsAuthApiConfig { ... }`).
- These can be replaced by small `use` statements at the top of each file (for the type and for Arc) and the type paths shortened.
- Risk is minimal; compile and run tests after changes to verify.

Initial scope (files to modify)
- [`src/graphql/resolvers/create_session.rs`](src/graphql/resolvers/create_session.rs:1)
- [`src/handlers/graphql.rs`](src/handlers/graphql.rs:1)

Example changes
- [`src/graphql/resolvers/create_session.rs`](src/graphql/resolvers/create_session.rs:1) — before:
```rust
let config = ctx.data::<std::sync::Arc<crate::DpsAuthApiConfig>>()?;
let test_config = crate::DpsAuthApiConfig {
  port: 0,
  // ...
};
```
- [`src/graphql/resolvers/create_session.rs`](src/graphql/resolvers/create_session.rs:1) — after (add imports at top of file):
```rust
use std::sync::Arc;
use crate::DpsAuthApiConfig;

let config = ctx.data::<Arc<DpsAuthApiConfig>>()?;
let test_config = DpsAuthApiConfig {
  port: 0,
  // ...
};
```
- [`src/handlers/graphql.rs`](src/handlers/graphql.rs:1) — before:
```rust
use crate::graphql::schema::AppSchema;
// ...
pub async fn graphql_post_handler(
  http_req: Request,
  config: Arc<crate::DpsAuthApiConfig>,
) -> impl IntoResponse {
```
- [`src/handlers/graphql.rs`](src/handlers/graphql.rs:1) — after (add imports at top of file):
```rust
use std::sync::Arc;
use crate::DpsAuthApiConfig;

pub async fn graphql_post_handler(
  http_req: Request,
  config: Arc<DpsAuthApiConfig>,
) -> impl IntoResponse {
```

Implementation steps
1. Update [`src/graphql/resolvers/create_session.rs`](src/graphql/resolvers/create_session.rs:1): add `use` imports and replace fully-qualified paths.
2. Update [`src/handlers/graphql.rs`](src/handlers/graphql.rs:1): add `use` imports and replace fully-qualified paths.
3. Run tests with: mise exec -- cargo test
4. Propagate the same pattern to other files found by the earlier search (tests, services, queries).
5. Run tests again and fix any fallout.

Risk and mitigations
- Missing imports may cause compile errors; mitigation: run tests and fix errors.
- Team style preferences; mitigation: keep changes minimal and document rationale.

Estimated effort
- ~15–30 minutes to implement the two files and run tests locally.

Deliverables
- Modified files: [`src/graphql/resolvers/create_session.rs`](src/graphql/resolvers/create_session.rs:1), [`src/handlers/graphql.rs`](src/graphql/resolvers/create_session.rs:1) plus any additional files updated.
- Test output.
- This plan file saved at [`docs/llm/plans/2025/11/16-use-crate-usage.md`](docs/llm/plans/2025/11/16-use-crate-usage.md:1).

Next
- I will proceed to implement these changes once you approve this plan.