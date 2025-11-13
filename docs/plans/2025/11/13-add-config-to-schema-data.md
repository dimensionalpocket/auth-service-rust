Title: Inject `ResolvedServerConfig` into async-graphql schema data
Date: 2025-11-13
Author: GitHub Copilot

Summary
- Describe a focused alternative: put the server configuration into the async-graphql schema data instead of passing fields through route closures.
- Explain exactly why an `Arc<ResolvedServerConfig>` should be used, in beginner-friendly Rust terms.
- Show the minimal code changes to implement the approach, and the code that would be removed.
- Explain impact on tests, runtime, and security.

Motivation
- Current code passes `cookie_domain`, `insecure_cookie`, and `session_secret` individually into the GraphQL POST handler and into the session middleware factory.
- The alternative places a single shared `ResolvedServerConfig` object into the async-graphql schema's data storage, making it accessible to resolvers via `Context::data::<T>()`.
- This can simplify call-sites and centralize configuration for resolvers that need it.

High-level change
- Wrap an instance of `ResolvedServerConfig` in `std::sync::Arc`, then call `.data(config.clone())` on the `SchemaBuilder` returned by `build_schema()`.
- Remove explicit parameter passing of `cookie_domain`, `insecure_cookie`, and `session_secret` from the `build_router`->`.post(...)` closure.
- Access the config from resolvers and (optionally) from handler code via schema data.

Why `Arc` (beginner-friendly explanation)
- Ownership basics: In Rust, a value has a single owner. When you want multiple parts of a program to share read-only access to the same value, you must use a sharing type.
- `Box<T>` gives heap allocation but still a single owner. `Rc<T>` allows multiple owners, but only for single-threaded scenarios.
- `async-graphql` and Axum handle requests concurrently across threads. `Rc<T>` is not safe to share across threads.
- `Arc<T>` (atomic reference-counted pointer) allows multiple owners across threads safely. Cloning an `Arc<T>` is cheap: it only increments a counter; it does not duplicate the entire `T`.
- Therefore, for a config object that needs to be shared across request handlers and/or resolvers in a multi-threaded server, `Arc<...>` is the right tool.
- Note: If the config must be mutated at runtime, you'd pair `Arc<T>` with interior mutability like `Arc<Mutex<T>>` or `Arc<RwLock<T>>`, but prefer immutable config if possible.

Practical constraints and trait bounds
- Many libraries require shared data to be `Send + Sync + 'static`. For `Arc<ResolvedServerConfig>` to be used as schema data and across threads, the inner `ResolvedServerConfig` must be `Send + Sync + 'static` (commonly satisfied for simple types like `String`, `Vec<u8>`, `bool`, and `u16`).
- Using `Arc` helps satisfy the requirements because `Arc<T>` implements `Send + Sync` when `T: Send + Sync`.

Minimal code changes (concrete example)

1) Build the `Arc` and add to schema data (in `src/dps_auth_api.rs`, inside `create_app`):

```rust
use std::sync::Arc;

pub async fn create_app(&self) -> Result<Router, DpsAuthApiError> {
  let database = self.initialize_database().await?;

  let config = Arc::new(self.config.clone());

  let schema = crate::graphql::schema::build_schema()
    .data(database.pool)
    .data(config.clone()) // inject config into schema data
    .finish();

  Ok(self.build_router(schema))
}
```

2) Simplify the GraphQL POST handler signature (in `src/handlers/graphql.rs`):

Before (example):
```rust
pub async fn graphql_post_handler(
  state: AppSchema,
  request: axum::http::Request<axum::body::Body>,
  cookie_domain: String,
  insecure_cookie: bool,
  session_secret: Vec<u8>,
) -> impl axum::response::IntoResponse {
  // existing logic
}
```

After:
```rust
pub async fn graphql_post_handler(
  state: AppSchema,
  request: axum::http::Request<axum::body::Body>,
) -> impl axum::response::IntoResponse {
  // resolver logic can access config from the schema's data context
}
```

3) Accessing config in resolvers (or other GraphQL code):

```rust
use std::sync::Arc;
use crate::dps_auth_api::ResolvedServerConfig;
use async_graphql::Context;

// inside a resolver function receiving `ctx: &Context<'_>`
let config_arc: &Arc<ResolvedServerConfig> = ctx.data::<Arc<ResolvedServerConfig>>();
let cookie_domain = &config_arc.cookie_domain;
let insecure = config_arc.insecure_cookie;
let secret = &config_arc.session_secret; // careful with usage
```

4) If handler-level (HTTP) code needs config before invoking GraphQL execution:
- You have two choices:
  - Capture an `Arc` clone in the route closure (this reintroduces a small local `Arc` capture but matches earlier behavior), or
  - Read the config from the `AppSchema`/schema data if `AppSchema` exposes an accessor (some schema wrappers allow `schema.data::<T>()`). If not, capturing `Arc` in the closure is simple and explicit.

Code that can be removed
- From `build_router` (remove these allocations/clones and parameter passing):

```rust
let cookie_domain = self.config.cookie_domain.clone();
let insecure_cookie = self.config.insecure_cookie;
let session_secret = self.config.session_secret.clone();

move |state, request| {
  crate::handlers::graphql::graphql_post_handler(
    state,
    request,
    cookie_domain.clone(),
    insecure_cookie,
    session_secret.clone(),
  )
}

.layer(from_fn(
  crate::middleware::session::create_session_middleware(self.config.session_secret.clone()),
))
```

- With the schema-data approach, the handler receives only `state` and `request`. Session middleware may still exist as an HTTP concern, but you can instead perform session handling inside resolvers using the config from schema data if desired.

Migration steps (ordered)
1. Add `use std::sync::Arc;` to relevant files.
2. In `create_app`, construct `let config = Arc::new(self.config.clone());` and inject it into the schema via `.data(config.clone())`.
3. Remove explicit config param passing from `build_router` `.post(...)` closure and update `graphql_post_handler` signature accordingly.
4. Update resolvers to read config using `ctx.data::<Arc<ResolvedServerConfig>>()` where needed.
5. Decide whether to keep or remove `create_session_middleware`:
   - Keep it: adjust it to use the captured `Arc` or to read from schema data if appropriate.
   - Remove it: ensure resolvers or GraphQL execution pipeline perform equivalent session parsing/validation.
6. Update tests: any test that constructs the handler or calls `graphql_post_handler` directly must be updated to use the new signature and to populate schema data when building a schema for testing.
7. Run tests and adjust any `Send/Sync/'static` issues.

Testing notes
- Unit tests that previously called `graphql_post_handler(state, request, ...)` must now call `graphql_post_handler(state, request)` and ensure the `state` (schema) contains the data injected via `SchemaBuilder::data(...)`.
- Create a small helper in tests to build a schema with `Arc::new(test_config)` and `database.pool` so tests can reuse it.

Security considerations
- `session_secret` is sensitive. Placing it into schema data makes it reachable from any resolver or code that has access to the GraphQL `Context`. Consider:
  - Minimizing places that read the secret.
  - Keeping secret usage limited to middleware/resolver helpers that validate/sign cookies or tokens.
  - Optionally split the config into `PublicConfig` and `SensitiveConfig` and only inject the sensitive part where strictly needed.

Pros of this approach (recap)
- Centralized config in one place for resolvers.
- Cleaner route handler signatures.
- Easy to add new config values later without changing many function signatures.
- `Arc` clones are cheap, reducing runtime cost.

Cons / trade-offs (recap)
- Larger security surface for secrets.
- Slight coupling of GraphQL layer to server config structure.
- Tests need small refactors.

Recommendation
- Use `Arc<ResolvedServerConfig>` injected into the schema if many resolvers need configuration or if you want a single source of truth inside GraphQL.
- If you need stronger secret isolation, split sensitive fields into a separate `SensitiveConfig` and only inject that where absolutely necessary.

Appendix: Quick `Arc` primer (for beginners)
- `Arc<T>` is like `Rc<T>` but thread-safe. Use `Arc` when data will be shared across threads.
- Cloning an `Arc<T>` is O(1) — it increments a reference count; it does not clone `T` itself.
- Example:

```rust
use std::sync::Arc;

let cfg = Arc::new(ResolvedServerConfig { /* fields */ });
let cfg2 = cfg.clone(); // cheap clone
// both cfg and cfg2 point to the same underlying data
```

- If you see compiler errors mentioning `Send` or `Sync`, it usually means a type inside `ResolvedServerConfig` does not satisfy thread-safety traits. Common primitive types like `String`, `Vec<u8>`, `bool`, and integers are fine.


---

If you want, I can now implement this specific change (injecting `Arc<ResolvedServerConfig>` into the schema, updating the handler signature, and adjusting tests). Which would you like me to do next: implement and run tests, or implement without running tests?