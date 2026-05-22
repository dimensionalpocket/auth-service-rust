# Sessions Table Structure (Session DB)

## Goal

Add a simple, industry-standard `sessions` table (SQLite) in the **separate session database** (with foreign keys disabled) to track user sessions so users can list and manage their active sessions.

Primary use cases:

- Create a session at login
- List a user's ongoing sessions (last access, browser/OS, country)
- Revoke a session (logout current session, or "log out other devices")
- Periodic cleanup of expired/revoked sessions

Non-goals (for now): refresh-token chains, device attestation, complex risk scoring.

## Best-Practice Notes (Keep It Simple)

Based on common web session guidance (e.g., OWASP Session Management Cheat Sheet):

- Use **high-entropy, meaningless** session identifiers (generate with a CSPRNG; avoid embedding user data).
- Store a **server-side verifier** for the session token (prefer a `token_hash`, not the raw token).
- Track **expiration** (`expires_ts`) and support **revocation** (`revoked_ts`).
- Track **idle activity** (`last_access_ts`) to support “last seen” and (optionally) idle timeout.
- Store only the **minimum metadata** needed for UX (UA-derived browser/OS, country). Avoid storing secrets/PII.
- Index by `user_id` for listing sessions and by `expires_ts` for cleanup.

## CSPRNG in Rust (Session IDs / Tokens)

Use the OS RNG (cryptographically secure) for any session identifiers or token material.

- `rand` crate: `rand::rngs::OsRng` + `RngCore::fill_bytes(...)`
- `getrandom` crate: low-level access to the OS RNG
- If using UUIDs: `uuid` crate v4 uses the OS RNG under the hood (via `getrandom`)

Example (random bytes):

```rust
use rand::rngs::OsRng;
use rand::RngCore;

let mut bytes = [0u8; 32];
OsRng.fill_bytes(&mut bytes);
```

## Proposed Table (Most Important Artifact)

Conventions aligned with the existing project migrations: timestamps are `INTEGER` seconds since Unix epoch.

```sql
-- Session DB: foreign_keys = OFF
CREATE TABLE sessions (
    -- Public identifier for UI / APIs (safe to expose).
    id TEXT PRIMARY KEY,

    -- User id from main DB (no FK enforced in this DB).
    user_id INTEGER NOT NULL,

    -- Verifier for the session cookie/token.
    -- Store a hash of the token (or of a stable token ID) so DB leakage
    -- does not immediately grant active sessions.
    token_hash TEXT NOT NULL UNIQUE,

    -- Standard lifecycle timestamps.
    created_ts INTEGER NOT NULL,
    updated_ts INTEGER NOT NULL,
    last_access_ts INTEGER,
    expires_ts INTEGER NOT NULL,
    revoked_ts INTEGER,

    -- Client metadata (best-effort; nullable).
    user_agent TEXT,
    browser_name TEXT,
    browser_version TEXT,
    os_name TEXT,
    os_version TEXT,
    country_code TEXT,

    -- Optional: useful for support/security; keep nullable to avoid hard dependency.
    ip_address TEXT
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_ts ON sessions(expires_ts);
CREATE INDEX idx_sessions_last_access_ts ON sessions(last_access_ts);
```

### Notes on Key Columns

- `id`: recommend a random UUID string (or similar) used for session listing/revocation APIs.
- `token_hash`: recommend hashing (e.g., SHA-256) of the session token (or token identifier) and comparing by hash.
- `last_access_ts`: updated on authenticated requests (consider throttling updates, e.g., at most once per N minutes).
- `revoked_ts`: set on logout/revoke; treat revoked sessions as invalid even if `expires_ts` is in the future.
- `country_code`: ISO 3166-1 alpha-2 (e.g., `US`, `DE`), derived from IP geo lookup at time of request.

## Query Patterns Enabled

- List sessions for user: `WHERE user_id = ? AND revoked_ts IS NULL AND expires_ts > now ORDER BY last_access_ts DESC NULLS LAST, created_ts DESC`
- Validate request session: lookup by `token_hash`, ensure not revoked/expired
- Revoke single session: `UPDATE sessions SET revoked_ts = now, updated_ts = now WHERE id = ? AND user_id = ?`
- Revoke all other sessions: `UPDATE sessions SET revoked_ts = now ... WHERE user_id = ? AND id != ? AND revoked_ts IS NULL`
- Cleanup job: `DELETE FROM sessions WHERE (revoked_ts IS NOT NULL AND revoked_ts < now - retention) OR expires_ts < now - retention`

## User-Agent Normalization Crates (Browser/OS)

Options to parse and normalize `user_agent` into browser + OS:

- `uaparser` (UA Parser regex-based; good normalization quality; depends on regex dataset updates)
- `woothee` (fast, lightweight; good for broad categories; may be less detailed than UA Parser)
- `user-agent-parser` (older UA Parser-style ecosystem; evaluate maintenance/accuracy before choosing)

Recommendation: start with `uaparser` if we want more standard browser/OS naming; use `woothee` if we prioritize speed and simplicity.

## References

- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP Authentication Cheat Sheet (session notes): https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
