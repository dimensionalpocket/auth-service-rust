Title: Pass full server config to GraphQL POST handler
Date: 2025-11-13
Author: GitHub Copilot

Summary
- Describe why `build_router` currently passes `cookie_domain`, `insecure_cookie`, and `session_secret` separately to `graphql_post_handler`.
- Explain pros and cons of passing the entire `ResolvedServerConfig` instead.
- Show exact impact on code, tests, and runtime behavior.
- Provide concrete code samples for how to implement the change (using `Arc<ResolvedServerConfig>`), and migration steps.

Motivation / Current behaviour
- Current code (in `src/dps_auth_api.rs`) constructs the GraphQL route and passes three specific fields:
  - `cookie_domain: String`
  - `insecure_cookie: bool`
  - `session_secret: Vec<u8>`

- Those fields are passed separately because the handler and the session middleware only require those specific values. This keeps handler/middleware signatures minimal and explicit.

Why the fields are passed individually (reasons)
- Minimal surface area: handlers receive only the exact data they need, reducing chance of accidental usage or leakage.
- Explicit API: function signatures document exactly what each handler consumes.
- Security containment: `session_secret` is sensitive; passing only that field to components that need it limits accidental exposure.
- Simpler trait/lifetime bounds: passing a couple of cloned values avoids making the whole `ResolvedServerConfig` satisfy additional sync/static bounds.
- Tests: simpler to mock/construct only the required small values.

Proposal: Pass the whole `ResolvedServerConfig` to the GraphQL POST handler
- Instead of passing three independent values, capture and pass `Arc<ResolvedServerConfig>` into the route closure and then into `graphql_post_handler` and the session middleware.

Implementation options (recommended)
- Use `Arc<ResolvedServerConfig>` to avoid expensive per-request cloning of strings/vecs.
- Update handler signature to accept `Arc<ResolvedServerConfig>` (or take references if using closure capture).
- Update `create_session_middleware` to accept `Arc<ResolvedServerConfig>` or to be created via a closure capturing the `session_secret` from the `Arc`.

Concrete code samples
- Small focused example showing changes in `build_router` (in `src/dps_auth_api.rs`):

```rust
use std::sync::Arc;

pub fn build_router(&self, schema: AppSchema) -> Router {
  // Wrap config in Arc once
  let config = Arc::new(self.config.clone());

  Router::new()
    // ... other routes ...
    .route(
      "/graphql",
      get({
        let development_mode = self.config.development_mode;
        move || crate::handlers::graphql::graphql_get_handler(development_mode)
      })
      .post({
        let config = config.clone();
        move |state, request| {
          crate::handlers::graphql::graphql_post_handler(
            state,
            request,
            config.clone(), // pass Arc<ResolvedServerConfig>
          )
        }
      })
      .layer(from_fn(
        // pass Arc into middleware factory; middleware can clone the Arc cheaply
        crate::middleware::session::create_session_middleware(config.clone()),
      )),
    )
    // ... fallback and other layers ...
    .with_state(schema)
}
```

- Example handler signature change (in `src/handlers/graphql.rs`):

```rust
use std::sync::Arc;
use crate::dps_auth_api::ResolvedServerConfig;

pub async fn graphql_post_handler(
  schema: AppSchema,
  request: axum::http::Request<axum::body::Body>,
  config: Arc<ResolvedServerConfig>,
) -> impl axum::response::IntoResponse {
  // Use config.cookie_domain, config.insecure_cookie, config.session_secret
}
```

- Example middleware factory change (in `src/middleware/session.rs`):

```rust
use std::sync::Arc;
use crate::dps_auth_api::ResolvedServerConfig;

pub fn create_session_middleware(
  config: Arc<ResolvedServerConfig>,
) -> impl Fn(axum::extract::RequestParts<()>) -> _ + Clone {
  // inside middleware closure, use config.session_secret.clone() as needed
}
```

Alternative: store config in `async-graphql` schema data
- `build_schema().data(database.pool).data(config.clone()).finish()`
- Pros: resolvers can access config via schema context; one central place for shared data.
- Cons: puts `session_secret` into resolver-visible context (security concern), and moves configuration into GraphQL schema lifetime.

Detailed implementation (schema data approach)

- How it works: add the `ResolvedServerConfig` (or an `Arc`-wrapped version) into the async-graphql schema using `SchemaBuilder::data`. That makes the config available inside GraphQL resolvers via `Context::data::<T>()`. The Axum request handler (the GraphQL POST handler) does not need to be passed the config explicitly — it can rely on the schema's contained data or perform the request execution without receiving extra params.

- Build-time change (in `src/dps_auth_api.rs`):

```rust
use std::sync::Arc;

let config = Arc::new(self.config.clone());
let schema = crate::graphql::schema::build_schema()
  .data(database.pool)
  .data(config.clone()) // <- inject config into schema data
  .finish();

Ok(self.build_router(schema))
```

- Handler changes (in `src/handlers/graphql.rs`):

1) Remove the extra parameters from the `graphql_post_handler` signature. Before:

```rust
pub async fn graphql_post_handler(
  state: AppSchema,
  request: axum::http::Request<axum::body::Body>,
  cookie_domain: String,
  insecure_cookie: bool,
  session_secret: Vec<u8>,
) -> impl axum::response::IntoResponse {
  // ...
}
```

After (no explicit config params):

```rust
pub async fn graphql_post_handler(
  state: AppSchema,
  request: axum::http::Request<axum::body::Body>,
) -> impl axum::response::IntoResponse {
  // When a resolver or handler needs config, access it from the schema data.
}
```

2) Inside resolvers or handler logic that needs configuration, read it from the schema's data context.

- In resolvers (async-graphql):

```rust
use std::sync::Arc;
use crate::dps_auth_api::ResolvedServerConfig;

// Inside a resolver function with `ctx: &Context<'_>`
let config = ctx.data::<Arc<ResolvedServerConfig>>();
let cookie_domain = &config.cookie_domain;
let secret = &config.session_secret;
```

- In the axum handler (outside resolvers) it's often preferable to avoid pulling config from the schema; instead, let the resolver use it. If the handler must access it (for example to set cookies on the HTTP response wrapper before/after GraphQL execution), you can use the schema's internal data by calling `state.data::<Arc<ResolvedServerConfig>>()` if the `AppSchema` type exposes that accessor, or keep a separate `Arc` captured in the route closure. The simplest route: keep cookie-setting logic inside resolvers or a small helper that receives the GraphQL response and the config from the schema data.

Code that would be removed as a result

- Direct parameter passing in `build_router` (remove these clones and params):

```rust
// Removed from build_router -> .post(...) invocation
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
```

- Middleware factory usage that explicitly required only the secret could be simplified or removed if middleware is implemented as a resolver-level concern. Example removed line(s):

```rust
.layer(from_fn(
  crate::middleware::session::create_session_middleware(self.config.session_secret.clone()),
)),
```

Note: removing the middleware factory like this is optional — many projects keep a lightweight HTTP-level session middleware (cookie parsing/verification) even when config lives in schema data. If you remove it, you must ensure that resolvers can still access session data (e.g., by reading cookies and validating them inside resolver helpers using `ctx.data::<...>()`).

Trade-offs for this approach (added detail)

- Single-source-of-truth: config is stored centrally in schema data, avoiding duplication and keeping resolvers consistent.
- Reduced route-signature complexity: handlers no longer need parameter lists for config fields.
- Security surface: resolver code (or any user-defined schema code) can now access `session_secret` if a developer accidentally calls `ctx.data::<Arc<ResolvedServerConfig>>()` — consider whether that risk is acceptable.

When to use this approach

- Good if many resolvers need access to configuration values, or if you want to inject environment-like values into the GraphQL layer cleanly.
- Avoid if you want strict separation between HTTP layer (cookie/middleware) and GraphQL resolver logic, or if minimizing secret exposure is a priority.

Pros of passing the entire config (summary)
- Fewer parameters across functions when multiple config values are required.
- Easier to add new config usage without changing handler signatures.
- Using `Arc` avoids repeated deep clones and keeps memory overhead small.
- Centralizes configuration access (handlers/middlewares get same coherent snapshot).

Cons / Risks (summary)
- Coupling: handlers and middleware become dependent on the entire config shape; changing `ResolvedServerConfig` will ripple across code.
- Security: more places might access `session_secret` accidentally, increasing risk of logging or accidental exposure.
- API clarity: function signatures no longer document which specific pieces of configuration are required.
- Trait + lifetime constraints: making the config available across async boundaries may require `Send + Sync + 'static` for the config and its contained fields.
- Tests: more test updates; creation of Arc-wrapped config instances in many tests.

Performance notes
- If you naively clone `ResolvedServerConfig` per request, cloning `Vec<u8>` (session_secret) and `String` repeatedly can be expensive.
- Use `Arc<ResolvedServerConfig>` to share one read-only copy across tasks cheaply.
- Ensure internal fields are not mutated; `Arc` only shares immutable data.

Security recommendations
- Avoid putting `session_secret` where business logic/resolvers that don't need it can access it. If you must pass full config, consider splitting sensitive fields into a smaller SensitiveConfig that is only passed to middleware that needs it, leaving non-sensitive fields in the public config.
- Ensure secrets are not logged or accidentally serialized in responses.

Files to change (explicit list)
- `src/dps_auth_api.rs` — wrap `self.config.clone()` in an `Arc` and pass into route closures + middleware factories.
- `src/handlers/graphql.rs` — change `graphql_post_handler` signature to accept `Arc<ResolvedServerConfig>` and update code accordingly.
- `src/middleware/session.rs` — accept config (or a SensitiveConfig) or use closure capture of `session_secret` from the Arc.
- `src/dps_auth_api_builder.rs` — may need no change, but tests and callers constructing `DpsAuthApi` should be updated to account for `Arc` usage if tests assert exact types.
- `src/*` unit/integration tests: update tests that call `graphql_post_handler` or construct route closures directly. Specifically update tests in `src/dps_auth_api.rs` (the test module) to create `Arc::new(server.config.clone())` where needed.

Migration steps (safe, ordered)
1. Add `use std::sync::Arc;` where necessary.
2. Change `build_router` to create `let config = Arc::new(self.config.clone());` and pass `config.clone()` to closures and middleware factory functions.
3. Update `graphql_post_handler` signature and body to accept `Arc<ResolvedServerConfig>`.
4. Update `create_session_middleware` factory to accept an `Arc` or to return a closure that captures the secret from the Arc.
5. Update tests to construct `Arc::new(server.config.clone())` and change direct handler calls to pass the Arc.
6. Run tests and adjust minor compile errors (likely lifetime/trait bounds or places that previously expected primitive args).

Example small migration diff (conceptual)
- Before (call site):
  - `graphql_post_handler(state, request, cookie_domain.clone(), insecure_cookie, session_secret.clone())`
- After (call site):
  - `graphql_post_handler(state, request, config.clone())`

Notes and trade-offs
- If you do not care about breaking changes (your stated preference), this change can be made across the codebase quickly.
- Prefer `Arc` to raw cloning for performance. If the config must be mutable at runtime, consider `Arc<Mutex<...>>`, but avoid this unless necessary.
- If minimizing exposure of `session_secret` is desired, create two objects:
  - `ServerConfig` with public fields (cookie domain, flags, etc.)
  - `SensitiveConfig` with `session_secret` only, only injected into middleware that needs it.

Recommendation
- If you expect the handlers or resolvers to need additional configuration fields in future, favor passing an `Arc<ResolvedServerConfig>` now (or add config to schema data) to reduce churn.
- If security isolation and minimal API surface are high priority, keep passing individual fields or split `ResolvedServerConfig` into `PublicConfig` and `SensitiveConfig`.

Deliverables in this plan
- This markdown plan file located at `docs/plans/2025/11/2025-11-13-pass-config-to-graphql-handler.md` (created here).
- Code samples in this plan that can be copy/pasted as patches for the implementation.

Questions for reviewer
- Do you prefer a single `Arc<ResolvedServerConfig>` approach, or would you like `SensitiveConfig`/`PublicConfig` separation to protect the secret more explicitly?
- Should the config be added to the async-graphql schema data instead of passing it through route closures?


