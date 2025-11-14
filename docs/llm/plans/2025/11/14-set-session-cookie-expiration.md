# Plan: Set session cookie expiration (revised)

(THIS PLAN HAS BEEN SCRAPPED. GPT5-mini GOT IT ALL WRONG.)

Purpose

- Update how the session cookie's Max-Age is determined so consumers can configure TTL via an env var or programmatic config, without modifying the external crate's create_payload.

Background

- The external payload creator DpsAuthSession::create_payload(user_id, expiration_seconds) lives in another crate. Passing None uses its internal default (currently 3 days). We must not change that crate; instead we should pass Some(ttl) when we want to override the default, or pass None to preserve the external default behavior.

Key decisions

- Env var name: DPS_AUTH_SESSION_TTL_SECONDS — integer seconds only (no human-friendly formats).
- Precedence: per-call parameter > programmatic config > env var > external default (None).
- Cookie Max-Age: when TTL is known use it; otherwise decode the issued token's exp claim to set cookie expiry. Avoid hardcoding Max-Age in resolvers.

Files to modify

- [`src/graphql/resolvers/create_session.rs:1`] — replace hardcoded Max-Age with computed value and use ttl_to_use when creating the cookie.
- [`src/services/session_service.rs:1`] — compute ttl_to_use and pass Some/None into external create_payload call.
- [`src/dps_auth_api_builder.rs:1`] — add/extend builder to accept session_ttl_seconds programmatically.
- [`src/dps_auth_api.rs:1`] — thread config into services.
- add helper: [`src/utils/token_exp.rs:1`] — decode exp claim from token.
- tests: add `tests/session_expiration_tests.rs` and unit tests.

Implementation details

1) Configuration surface
- Add DpsAuthConfig with fields:
  - session_ttl_seconds: Option<u64> (None means use env or external default)
  - rolling_sessions: bool
- Wire the config into [`src/dps_auth_api_builder.rs:1`].
- At startup read DPS_AUTH_SESSION_TTL_SECONDS and parse it as u64 seconds; only accept integer seconds.

2) TTL selection logic (in [`src/services/session_service.rs:1`])
- Compute ttl_to_use like:
```rust
let ttl_to_use: Option<u64> = session_param_ttl
    .or(config.session_ttl_seconds)
    .or(env_ttl_seconds);
let payload = DpsAuthSession::create_payload(user.id, ttl_to_use);
let token = DpsAuthSession::encode_token(&payload, secret)?;
```
- Important: pass Some(ttl) only when an override is present; otherwise pass None to preserve external default.

3) Cookie creation (in [`src/graphql/resolvers/create_session.rs:1`])
- Replace hardcoded Max-Age with computed cookie_max_age_secs:
  - If ttl_to_use.is_some() => cookie_max_age_secs = ttl_to_use.unwrap()
  - Else decode exp from token to compute remaining seconds via helper
  - If decoding fails, fallback to the documented default (3 days) but do not change external behavior
- Example:
```rust
let cookie_max_age_secs = match ttl_to_use {
    Some(s) => s,
    None => decode_exp_from_token(&token).unwrap_or(3 * 24 * 60 * 60),
};

let cookie = Cookie::build("dps_session", token)
    .path("/")
    .http_only(true)
    .secure(config.secure_cookies)
    .same_site(SameSite::Lax)
    .max_age(time::Duration::seconds(cookie_max_age_secs as i64))
    .finish();
```

4) Token exp decoder helper
- Implement [`src/utils/token_exp.rs:1`] with:
  - fn decode_exp_from_token(token: &str) -> Option<u64>
- Use a JWT parsing library to read the exp claim safely. Return remaining seconds between now and exp.
- If signature verification is expensive or secret not available, parse unverified for exp only but document tradeoffs.

5) Validation and clamping
- When reading TTL from env or config: parse to u64 and clamp to min = 60 (1 minute), max = 31_536_000 (1 year). Log WARN when clamped.
- Reject non-integer values; treat them as absent.

Tests

- Unit:
  - verify ttl_to_use resolution (per-call, config, env, default).
  - verify decode_exp_from_token returns correct remaining seconds.
- Integration:
  - verify cookie Max-Age matches TTL for per-call/config/env.
  - verify cookie Max-Age derived from token exp when no override is present.
- Edge cases: invalid env var, overflow values, zero.

Documentation

- Update README to document:
  - DPS_AUTH_SESSION_TTL_SECONDS (seconds only).
  - Precedence order and that passing None leaves external crate default in effect.
  - Programmatic configuration via builder.
- Update this plan file.

Rollout

- Implement changes in a feature branch, add tests.
- Run full test suite with mise exec -- cargo test.
- Release with changelog noting the env var and behavior.

Security

- Do not log tokens or secrets.
- Clamp TTLs; keep rolling sessions opt-in and documented.
- Be conservative in defaults and avoid increasing default session lifetime.

Actionable todo list (next steps)

- Read README to confirm current default is documented.
- Add DpsAuthConfig and builder option for session_ttl_seconds.
- Read `DPS_AUTH_SESSION_TTL_SECONDS` at startup and parse to u64.
- Update [`src/services/session_service.rs:1`] to compute ttl_to_use and pass to create_payload.
- Update [`src/graphql/resolvers/create_session.rs:1`] to use cookie_max_age_secs instead of hardcoded value.
- Add [`src/utils/token_exp.rs:1`] and unit tests.
- Add integration tests `tests/session_expiration_tests.rs`.
- Update README and this plan file.

Approve?

- If approved I'll implement the changes in code mode and run tests.