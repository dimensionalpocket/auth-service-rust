- session sub to string + custom function to retrieve user
- session management (separate database)
- captcha on registration and login (cloudflare turnstile)
- emails (needs planning)

# Session Sub to Custom String

- `dps-auth-session` crate has been updated to 0.3.0 with breaking changes to the `SessionPayload` struct. The `sub` property, which was previously an integer, has been converted to a string. Also, the repository URL has changed to remove the `-rs` suffix. Update the URL and tag in the `Cargo.toml` file then update the code to accommodate the new string type for `sub`.
- SessionPayload (from DpsAuthSession crate) `pub` property has been converted to a string (breaking change) in the latest version. The crate needs to be updated to tag 0.3.0. See release notes here: https://github.com/dimensionalpocket/dps-auth-session/pull/10
- After this update, the build will probably break. SessionContext's `user_id()` method is returning an integer (the old `sub` type), but it should be updated to return a string.
