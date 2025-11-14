# Plan: Remove `ResponseHeaders` container and use async-graphql header APIs

Date: 2025-11-13

Goal
- Replace the current custom `ResponseHeaders` container + copying logic with `async-graphql`'s built-in `insert_http_header` / `append_http_header` APIs so resolvers write headers directly into the GraphQL response. This simplifies code and removes manual header-copying and locking.

Why
- Current approach: `graphql_post_handler` creates a `ResponseHeaders { headers: Arc<Mutex<HeaderMap>> }`, places `response_headers.headers.clone()` into the GraphQL request data, resolvers lock and mutate that shared header map, then after `schema.execute(request).await` the handler copies the stored headers into the outgoing HTTP response.
- Simpler approach: resolvers call `ctx.insert_http_header` / `ctx.append_http_header` and the `GraphQLResponse` produced by `async-graphql` already contains those headers; converting to an HTTP response applies them automatically.

Files touched (plan only — no code changes yet)
- `src/handlers/graphql.rs` — remove `ResponseHeaders` struct and associated creation, remove `request.data(response_headers.headers.clone())`, remove the post-response copy of headers into `http_response`.
- `src/graphql/resolvers/create_session.rs` — replace usage of `ResponseHeaders`/`set_session_cookie` with `ctx.append_http_header("set-cookie", cookie_value)` (or use `ctx.insert_http_header` when appropriate).
- `src/graphql/resolvers/*` — search for any other resolvers using `ResponseHeaders` and update them similarly.
- Tests under `src/graphql/resolvers/*` or integration tests that asserted on `ResponseHeaders` must be updated to call the GraphQL handler and assert on HTTP response headers (or on `GraphQLResponse` header metadata).

Detailed steps

1) Review the codebase for `ResponseHeaders` usage
- Find all references to `ResponseHeaders` (type, creation, `.data(...)`, and usages in resolvers). Confirm scope of changes.

2) Update `src/handlers/graphql.rs` (handler-side changes)
- Remove `ResponseHeaders` struct definition (if defined in this file) and any references.
- Remove creation of `response_headers` and the `request = request.data(response_headers.headers.clone());` call.
- Remove the block that locks `response_headers.headers` and copies headers into `http_response` after GraphQL execution.
- Keep `schema.execute(request).await` as-is — async-graphql will carry headers inserted by resolvers.

3) Update resolver(s) to use async-graphql header APIs
- Replace code which used `ctx.data::<Arc<Mutex<HeaderMap>>>()` and `set_session_cookie(...)` with logic that builds the cookie string and calls:
  - `ctx.append_http_header("set-cookie", cookie_value)` for Set-Cookie
  - or `ctx.insert_http_header("Header-Name", "value")` for single-value headers
- Example change is in `src/graphql/resolvers/create_session.rs` (build cookie_value and call `ctx.append_http_header("set-cookie", cookie_value)`)

4) Remove helper `set_session_cookie` (or keep if still used elsewhere)
- If `set_session_cookie` only exists to write into `ResponseHeaders`, either remove it or refactor to return the cookie string so resolvers can call `ctx.append_http_header("set-cookie", cookie_value)`.

5) Update tests
- Any unit tests that inspected the `ResponseHeaders` container should be updated to perform a real HTTP request via the handler (or to use `async_graphql` testing helpers to inspect response extensions) and assert on the final HTTP response headers.
- Add tests that ensure `Set-Cookie` appears in the HTTP response headers after running the GraphQL POST handler for `createSession`.

6) Run tests and iterate
- Run the test suite and fix failing tests.
- Pay particular attention to integration tests that relied on response header copying or direct access to `ResponseHeaders`.

Risks and mitigation
- Cookie semantics: `Set-Cookie` sometimes requires special handling by proxies or framework layers. Test in local dev with typical frontends and proxies.
- Ordering: If any other middleware needs to inspect headers placed by resolvers, confirm ordering. In our current flow resolvers add headers during execution; the handler converts GraphQL response to HTTP response after execution so downstream middleware should see final headers when the HTTP response is being sent.

Testing guidance
- Update existing tests to assert on the HTTP response headers returned from `graphql_post_handler`.
- Add an integration test that:
  1. Builds a `DpsAuthApi` with a test database and session secret.
  2. Calls `/graphql` POST using `tower::ServiceExt::oneshot` with a `createSession` mutation.
  3. Asserts response status is `200 OK` and response headers include `Set-Cookie` with expected prefix/name/value.

Estimated effort
- Code changes: ~30–90 minutes depending on number of resolvers using `ResponseHeaders`.
- Tests: ~30–60 minutes depending on current test coverage and required updates.

Rollback plan
- Keep the plan small and change one resolver at a time. If tests fail badly, revert the handler and resolver changes and re-introduce `ResponseHeaders` until issues are addressed.

Decision point (do not implement yet)
- This plan is ready to implement. I will not modify code until you approve the plan. When you approve, I can prepare a patch that:
  - Removes `ResponseHeaders` wiring in `src/handlers/graphql.rs`.
  - Updates `src/graphql/resolvers/create_session.rs` to use `ctx.append_http_header("set-cookie", ...)` and returns the same logical behavior.
  - Updates tests accordingly and run the test suite.

If you approve, which option do you prefer?
- I prepare a single patch that changes handler + `create_session.rs` + tests in one commit.
- I prepare smaller incremental patches: first handler only (no behavior change in resolvers), then update resolvers one-by-one, then adjust tests.


---

End of plan.
