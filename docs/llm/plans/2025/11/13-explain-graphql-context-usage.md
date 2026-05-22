# Explain GraphQL Context Usage in Resolvers

Date: 2025-11-13

Summary
- Goal: Explain how GraphQL `Context` (the `.data(...)` values) are provided to resolvers in this codebase, with concrete examples and sample code.
- Outcome: A short reference that shows where values are attached (schema-level and request-level), how middleware contributes, and how resolvers consume them.

Files referenced
- `src/graphql/schema.rs` (provides the schema builder)
- `src/dps_auth_api.rs` (attaches database pool to the schema)
- `src/handlers/graphql.rs` (attaches per-request data like session context, cookie settings, and session secret)
- `src/middleware/session.rs` (produces `SessionContext` and injects it into the HTTP request)
- `src/graphql/resolvers/create_session.rs` (example resolver that reads data from the GraphQL context)

High-level overview

1. Two places provide context data to resolvers:
   - Schema-level `.data(...)` on `SchemaBuilder` (global / application-scoped values). These are available to every request handled by that schema.
   - Request-level `.data(...)` on `async_graphql::Request` (per-request values). These are added by the HTTP handler before executing the request and can carry request-specific data (session info, headers container, per-request secret clones, etc.).

2. The HTTP middleware may also attach information to the Axum `Request` extensions. The GraphQL handler reads those extensions and then copies the relevant information into the GraphQL `Request` as per-request data.

Concrete code flow (where the context comes from)

A. Schema builder (global data)

- `src/graphql/schema.rs` defines `build_schema()` which returns a `SchemaBuilder<Query, Mutation, EmptySubscription>`.

```rust
// src/graphql/schema.rs
pub fn build_schema() -> async_graphql::SchemaBuilder<Query, Mutation, EmptySubscription> {
  Schema::build(Query::new(), Mutation::new(), EmptySubscription)
}
```

- At startup (or when creating the app/router for tests), the server supplies global data on the builder. Example from `create_app()`:

```rust
// src/dps_auth_api.rs (simplified)
let database = self.initialize_database().await?;
let schema = crate::graphql::schema::build_schema()
  .data(database.pool) // <-- schema-level data: SqlitePool
  .finish();
```

- `database.pool` (a `SqlitePool`) is therefore stored in the schema and resolvers can access it with `ctx.data::<SqlitePool>()`.

B. Request-level data added in the HTTP handler

- `src/handlers/graphql.rs` handles incoming HTTP GraphQL POSTs. It:
  1. Extracts `SessionContext` (provided by the session middleware) from request extensions.
  2. Creates a `ResponseHeaders` container (an `Arc<Mutex<HeaderMap>>`) for resolvers to populate headers.
  3. Parses the incoming HTTP body into an `async_graphql::Request`.
  4. Adds per-request data by calling `request = request.data(...)` for each item.

Key snippet (simplified):

```rust
// src/handlers/graphql.rs (simplified)
let session_context = http_req.extensions().get::<SessionContext>().cloned().unwrap_or_else(|| SessionContext::new(None));
let response_headers = ResponseHeaders { headers: Arc::new(Mutex::new(HeaderMap::new())) };
let mut request = graphql_request;
request = request.data(session_context);
request = request.data(response_headers.headers.clone());
request = request.data(cookie_domain);
request = request.data(insecure_cookie);
request = request.data(session_secret);

let response = schema.execute(request).await;
```

- Note: `cookie_domain`, `insecure_cookie`, and `session_secret` originate from `DpsAuthApi::build_router` where the closure passed to the handler captures these config values and passes them into `graphql_post_handler`.

C. Middleware: how `SessionContext` gets into the request

- `src/middleware/session.rs` defines `create_session_middleware(secret: Vec<u8>)` that returns an Axum middleware function.
- The middleware inspects headers and cookies, attempts to decode and validate the session token using `DpAuthSessionService`, and then inserts a `SessionContext` instance into the HTTP `Request` extensions:

```rust
// src/middleware/session.rs (simplified)
let session_context = extract_and_validate_session_sync(&request, &secret);
request.extensions_mut().insert(session_context);
```

- The GraphQL handler later reads that extension and moves the `SessionContext` into the GraphQL `Request` data (see handlers code above).

D. How resolvers consume context data

- Inside any resolver you can retrieve context data with `ctx.data::<T>()` (which returns a `Result<&T, _>`) or `ctx.data_opt::<T>()` for an Option.

Example from `src/graphql/resolvers/create_session.rs` (already in repo):

```rust
let pool = ctx.data::<SqlitePool>()?;         // schema-level database pool
let session_secret = ctx.data::<Vec<u8>>()?;  // per-request session secret (cloned into request.data)

// Optionally get the response headers container
if let Ok(response_headers) = ctx.data::<Arc<Mutex<HeaderMap>>>() {
  let default_domain = ".api.dps.localhost".to_string();
  let cookie_domain = ctx.data::<String>().unwrap_or(&default_domain);
  let insecure_cookie = *ctx.data::<bool>().unwrap_or(&false);
  set_session_cookie(response_headers, &token, cookie_domain, insecure_cookie);
}
```

Notes & gotchas

- Ownership and cloning:
  - `.data(x)` takes ownership of `x` and stores it in the SchemaBuilder or Request. When adding per-request values the handler often passes cloned values (e.g., `session_secret.clone()` in the router) so each request gets its own owned copy.
  - Types stored in schema-level data must be `Send + Sync + 'static`.

- Where to look for missing data:
  - If a resolver calls `ctx.data::<T>()` but the data wasn't attached, the resolver will return an error. That can happen if you forget to `.data(...)` at schema creation or forget to add a per-request `.data(...)` in the HTTP handler.

- Difference between Axum `State` and Async-GraphQL `.data`:
  - `Axum::State` lives on the Axum application/router and is passed to handlers via extractor. In this repo the `schema` itself is placed into Axum state via `.with_state(schema)` when building the router.
  - Async-GraphQL `.data(...)` stores typed values inside the GraphQL schema or the GraphQL request. Resolvers access these via `ctx.data::<T>()`.

Quick checklist for adding new context values

- If the data is application-global (database pool, config that is safe to reuse), attach it to the schema with `build_schema().data(value)` before `.finish()`.
- If the data is request-specific (authenticated session payload, per-request secret, headers container), add it to the `async_graphql::Request` in the HTTP handler using `request.data(value)`.
- If the data originates from middleware, ensure the middleware places it on the Axum `Request` extensions and then the HTTP handler copies it into the GraphQL `Request`.

Example summary (end-to-end):

- Startup: `build_schema().data(database.pool).finish()` → Schema now contains `SqlitePool`.
- Middleware: `create_session_middleware(secret)` decodes session token and places `SessionContext` in request extensions.
- Request handler: reads extension `SessionContext`, creates `ResponseHeaders`, and calls `request.data(...)` for each item including `session_secret` and `cookie_domain`.
- Resolver: calls `ctx.data::<SqlitePool>()` and `ctx.data::<SessionContext>()` (or `SessionContext::from_context(ctx)`) to read the values and operate.

Naming multiple values of the same Rust type (e.g. multiple SqlitePool instances)
-----------------------------------------------------------------------------

Problem:
- `async_graphql` stores context values by Rust type. Calling `.data(database.pool)` and later `ctx.data::<SqlitePool>()` works when you only have one `SqlitePool` in the context. If you want to attach multiple `SqlitePool` instances (for example a `primary` and a `replica`), you can't store two values of the same concrete type because retrieval is type-based.

Recommended solution — newtype wrappers:
- Create small distinct wrapper types (newtypes) around `SqlitePool` so each pool has a unique Rust type. The GraphQL context keys are then the wrapper types.

Example:

```rust
// src/database/pools.rs
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct PrimaryPool(pub SqlitePool);

#[derive(Clone)]
pub struct ReplicaPool(pub SqlitePool);
```

Register them on the schema (or request) as before:

```rust
let schema = build_schema()
  .data(PrimaryPool(database.primary_pool.clone()))
  .data(ReplicaPool(database.replica_pool.clone()))
  .finish();
```

Consume them in resolvers by type:

```rust
use crate::database::pools::{PrimaryPool, ReplicaPool};

let primary = ctx.data::<PrimaryPool>()?.0.clone();
let replica = ctx.data::<ReplicaPool>()?.0.clone();
// use `primary` and `replica` as SqlitePool instances
```

Notes on wrappers:
- Keep wrapper types lightweight (single-field tuple structs) and `Clone` so they are ergonomic to use and cheap to pass around.
- You can implement `Deref<Target = SqlitePool>` for the wrapper to avoid `.0` access in resolvers.

Alternative approaches
- Named map: Store a `HashMap<String, SqlitePool>` inside a single wrapper type (e.g. `PoolsMap`) and look up by string key at runtime. This trades compile-time type-safety for runtime flexibility.

Example (map-based):

```rust
use std::collections::HashMap;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct PoolsMap(pub HashMap<String, SqlitePool>);

// register
let mut map = HashMap::new();
map.insert("primary".to_string(), database.primary_pool.clone());
map.insert("replica".to_string(), database.replica_pool.clone());
let schema = build_schema().data(PoolsMap(map)).finish();

// use
let pools = ctx.data::<PoolsMap>()?;
let primary = pools.0.get("primary").expect("missing primary pool");
```

- Use this when you truly need dynamic names or an unknown number of pools at compile time. Prefer newtypes when you know the set of pools at compile time because they give stronger typing and clearer code.

Summary recommendation
- Prefer newtype wrappers for each distinct `SqlitePool` you expect to use (`PrimaryPool`, `ReplicaPool`, etc.). They are simple, compile-time safe, and integrate cleanly with `async_graphql`'s type-based context.
- Use a `PoolsMap` only if the set of pools is dynamic or configured at runtime and you need lookup-by-name behavior.

Async-graphql header modification capabilities
-----------------------------------------------------------------------------

Async-graphql has a built-in feature to change headers:

```rust
#[Object]
impl Query {
    async fn greet(&self, ctx: &Context<'_>) -> String {
        // Headers can be inserted using the `http` constants
        let was_in_headers = ctx.insert_http_header(ACCESS_CONTROL_ALLOW_ORIGIN, "*");

        // They can also be inserted using &str
        let was_in_headers = ctx.insert_http_header("Custom-Header", "1234");

        // If multiple headers with the same key are `inserted` then the most recent
        // one overwrites the previous. If you want multiple headers for the same key, use
        // `append_http_header` for subsequent headers
        let was_in_headers = ctx.append_http_header("Custom-Header", "Hello World");

        String::from("Hello world")
    }
}
```

(write your answer here)

---

Files that would be modified by an implementation (none required for this plan)
- This plan is documentation-only. No code changes required.

End of plan.
