# Pass DpsAuthApiConfig into GraphQL request data

Summary:
- Replace per-field context insertion (cookie_domain, insecure_cookie, session_secret) with a single config object: DpsAuthApiConfig.
- Update router, handler, and resolvers/tests.

Decisions (from user):
- Keep middleware receiving session_secret directly; do not change middleware signature. DpsAuthApiConfig.session_secret is expected to be Vec<u8> so it can be passed as-is to middleware creation.
- Use Arc for sharing config in request.data; explanation and examples included below.
- Store config as Arc<DpsAuthApiConfig> on DpsAuthApi.

Background:
The current GraphQL POST handler receives cookie_domain, insecure_cookie, and session_secret passed from the router in [`src/dps_auth_api.rs:178`](src/dps_auth_api.rs:178).
We want resolvers to access the API's .config via the context and read its fields directly instead of receiving separate context data.

Goals:
1. Pass only (state, request, config) to `graphql_post_handler`.
2. Insert config into async-graphql request.data(...) so resolvers can access it.
3. Update resolvers that relied on separate context data to read the config and access its fields directly.
4. Keep middleware unchanged (session_secret passed directly).

Files to change:
- [`src/dps_auth_api.rs:178`](src/dps_auth_api.rs:178)
- [`src/handlers/graphql.rs:22`](src/handlers/graphql.rs:22)
- [`src/graphql/resolvers/create_session.rs:37`](src/graphql/resolvers/create_session.rs:37)
- Tests in [`src/graphql/resolvers/create_session.rs:116`](src/graphql/resolvers/create_session.rs:116)

Arc usage and where:
- Rationale: config is initialized once and read-only. Using Arc avoids cloning inner fields per request and ensures the value is Send + Sync for async-graphql.
- Decision: set `DpsAuthApi.config: Arc<DpsAuthApiConfig>` so code can cheaply clone the Arc where needed.
- Performance: removes runtime wrapping and allocations; Arc clone is cheap and avoids cloning strings/vecs per request.
- Safety: Arc<T> is thread-safe; Arc<DpsAuthApiConfig> is Send+Sync if fields are Sync.
- Migration surface: more invasive (must update constructor and call sites) but clean for long-lived server.
- Developer ergonomics: easier to capture `self.config.clone()` in closures and pass to handlers.

Code samples

Router wiring change (example):
```rust
// assuming `self.config: Arc<DpsAuthApiConfig>`
.post({
  let config = self.config.clone();
  move |state, request| {
    crate::handlers::graphql::graphql_post_handler(state, request, config.clone())
  }
})
```

Handler signature and request.data insertion (example):
```rust
pub async fn graphql_post_handler(
  State(schema): State<AppSchema>,
  http_req: Request,
  config: std::sync::Arc<crate::DpsAuthApiConfig>,
) -> impl IntoResponse {
  // ...
  request = request.data(config.clone());
  // remove previous request.data(cookie_domain/insecure_cookie/session_secret)
}
```

Resolver change (create_session) example:
```rust
let config = ctx.data::<std::sync::Arc<crate::DpsAuthApiConfig>>()?;
let session_secret = config.session_secret.clone(); // Vec<u8>
let cookie_domain = config.cookie_domain.clone();
let insecure_cookie = config.insecure_cookie;
```

Tests update sample:
```rust
let test_config = crate::DpsAuthApiConfig {
  port: 0,
  sqlite_main_file_path: "test.db".to_string(),
  session_secret: TEST_SECRET.to_vec(), // must be 32 bytes
  cookie_domain: ".api.test".to_string(),
  insecure_cookie: false,
  development_mode: true,
  sqlite_main_pool_size: 1,
};
let schema = Schema::build(...).data(pool).data(std::sync::Arc::new(test_config)).finish();
```

Migration checklist for Arc<DpsAuthApiConfig> approach:
- Update struct declaration at [`src/dps_auth_api.rs:8`](src/dps_auth_api.rs:8) to:
  `pub(crate) config: std::sync::Arc<DpsAuthApiConfig>`
- Update `DpsAuthApi::new` to wrap the built config with `Arc::new(config)`.
- Update any code that accessed `.config` fields directly to either clone the Arc and deref.
- Update tests and helpers that construct `DpsAuthApi` to use `.new` (if they don't already) so that the config can be automatically wrapped.

Middleware note:
- Leave `create_session_middleware(self.config.session_secret.clone())` unchanged as requested.
- `DpsAuthApi::new` already fills `session_secret` from `DpsConfig.get_auth_api_session_secret_bytes()`, which returns a `Vec<u8>`.
- If tests construct `DpsAuthApiConfig` directly, ensure `session_secret` is a valid 32-byte Vec<u8>.

Implementation steps (condensed):
1. Ensure `DpsAuthApiConfig` derives Clone + Debug and is accessible so code/tests can construct/read fields.
2. Apply Arc migration: change `DpsAuthApi.config` type to `Arc<DpsAuthApiConfig>` and update `DpsAuthApi::new`.
3. Wire router and closures to `self.config.clone()` and pass Arc into handlers.
4. Change handler signature to accept `Arc<DpsAuthApiConfig>` and insert it into async-graphql request.data.
5. Update `create_session` resolver to read the Arc-wrapped config from context and access fields directly.
6. Update tests to provide `Arc<DpsAuthApiConfig>` instead of raw session_secret/data primitives.
7. Run tests with the project environment: `mise exec -- cargo test`.

Validation checklist:
- [ ] Handler compiles and inserts config into request.data.
- [ ] `create_session` resolver reads config and builds cookie using fields.
- [ ] Middleware unchanged and tests updated.
- [ ] All async-graphql data types are Send + Sync + 'static.

Notes about types:
- async-graphql requires data to be Send + Sync + 'static. `Arc<T>` is Send + Sync if `T` is Sync.
- `String`, `Vec<u8>`, and `bool` are Sync, so `DpsAuthApiConfig` composed of them will be Sync.
- Using Arc avoids cloning `String`/`Vec<u8>` contents per request; Arc clone is cheap.

Follow-up:
After your approval I will implement the changes and run the test suite. Any deviation from this plan during implementation will be reported immediately.