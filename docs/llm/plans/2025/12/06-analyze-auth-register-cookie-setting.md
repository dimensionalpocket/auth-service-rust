# Analyze authRegister Resolver Cookie Setting Issue

**Date:** 2025-12-06@15:55  
**Issue:** authRegister resolver is not setting session cookie on successful registration

## Problem Analysis

Based on code review, the `authRegister` resolver in `src/graphql/resolvers/auth_register.rs` does not set a session cookie after successful user registration, unlike the `authLogin` resolver which properly sets the cookie.

### Current Behavior
- `authRegister` calls `AuthService::register()` which returns `RegisterResult` (user info only)
- No session is created during registration
- No cookie is set in the response
- User must manually log in after registration

### Expected Behavior
- After successful registration, a session should be automatically created
- Session cookie should be set in the response (same as login)
- User should be immediately authenticated

## Root Cause Analysis

1. **AuthService::register()** only creates user, no session creation
2. **AuthRegisterResolver** doesn't create session or set cookie
3. **AuthLoginResolver** properly creates session and sets cookie via `ctx.append_http_header()`
4. **SessionService::create_session()** exists and works - it looks up user by username, verifies password, and creates token

## How Session Creation Should Work

**Current `SessionService::create_session()` method:**
1. Takes username, password, and secret as parameters
2. Looks up user by username in database
3. Verifies password against stored hash using `PasswordService::verify()`
4. Creates session payload with user ID using `DpsAuthSession::create_payload()`
5. Encodes payload into token using `DpsAuthSession::encode_token()`

**Proposed new method `SessionService::create_session_for_user()`:**
1. Takes user object and secret as parameters (no password needed)
2. Creates session payload with user ID using `DpsAuthSession::create_payload()`
3. Encodes payload into token using `DpsAuthSession::encode_token()`

**Refactored `create_session()` method:**
1. Takes username, password, and secret as parameters
2. Looks up user by username and verifies password (existing logic)
3. Calls `create_session_for_user(&user, secret)` internally
4. Returns the token

**For registration flow:**
1. Create the user first (hashes password, stores in DB)
2. Call `SessionService::create_session_for_user(&user, session_secret)` directly
3. No password verification needed since user was just created successfully

## Implementation Plan

### Phase 1: Update SessionService ✅ COMPLETED
- ✅ Add new method `create_session_for_user(&user: User, secret: &[u8]) -> Result<String, SessionError>`
- ✅ This method creates session token for existing user without password verification
- ✅ Refactor existing `create_session()` to use this new method internally
- ✅ Add tests for new method
- ✅ All tests pass (9 session service tests + auth service tests + auth login tests)
- ✅ Code passes clippy and formatting checks

### Phase 2: Update AuthService ✅ COMPLETED
- ✅ Modify existing `register()` method to create session after successful user registration
- ✅ Add `session_secret: &[u8]` parameter to `register()` method signature
- ✅ After successful user creation via `UserService::create_user()`, call `SessionService::create_session_for_user(&user, session_secret)`
- ✅ Update `RegisterResult` struct to include `session_token` field (for resolver to use)
- ✅ Add `SessionError` variant to `UserError` enum
- ✅ All auth service tests pass

### Phase 3: Update AuthRegisterResolver ✅ COMPLETED
- ✅ Import required modules: `SESSION_COOKIE_NAME`, `DpsAuthApiConfig`
- ✅ Get config from context (same as authLogin)
- ✅ Call updated `AuthService::register()` with session secret
- ✅ Set session cookie using `ctx.append_http_header()` (same pattern as authLogin)
- ✅ `AuthRegisterResponse` unchanged (no token field in GraphQL response)
- ✅ Add `SessionError` handling in resolver
- ✅ All resolver tests pass

### Phase 4: Update Tests ✅ COMPLETED
- ✅ Add test for cookie setting in authRegister resolver
- ✅ Update existing resolver tests to handle config and cookie setting
- ✅ Add comprehensive cookie header validation test
- ✅ All 303 tests pass
- ✅ Code passes clippy and formatting checks

## Files to Modify

1. **src/services/session_service.rs** ✅ COMPLETED
   - ✅ Add new `create_session_for_user()` method
   - ✅ Refactor `create_session()` to use new method internally
   - ✅ Add tests for new method

2. **src/services/auth_service.rs**
   - Modify existing `register()` method signature to include session_secret parameter
   - Update `RegisterResult` struct to include session_token field
   - Add session creation logic after user creation
   - Update all callers of `register()` method

3. **src/graphql/resolvers/auth_register.rs**
   - Add imports for cookie handling and config
   - Update resolver to get config from context and pass session secret to service
   - Use returned `session_token` to set cookie via `ctx.append_http_header()`
   - `AuthRegisterResponse` unchanged (no token field in GraphQL response)
   - Update tests

4. **README.md**
   - No changes needed (authRegister response fields unchanged)

## Implementation Details

### New SessionService Method
```rust
impl SessionService {
    /// Create a session token for an existing user (no password verification)
    pub fn create_session_for_user(
        user: &crate::models::user::User,
        secret: &[u8],
    ) -> Result<String, SessionError> {
        let payload = DpsAuthSession::create_payload(user.id, None);
        let token = DpsAuthSession::encode_token(&payload, secret)?;
        Ok(token)
    }
}
```

### Refactored create_session Method
```rust
pub async fn create_session(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    secret: &[u8],
) -> Result<String, SessionError> {
    // Existing user lookup and password verification logic...
    let user = /* lookup and verify user */;
    
    // Use new method for token creation (no .await needed since it's sync)
    Self::create_session_for_user(&user, secret)
}
```

### Updated AuthService Method Signature
```rust
pub async fn register(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    password_confirmation: &str,
    session_secret: &[u8],  // New parameter
) -> Result<RegisterResult, UserError>
```

### Updated RegisterResult Struct
```rust
#[derive(Debug, Clone)]
pub struct RegisterResult {
    pub user_id: i64,
    pub username: String,
    pub uuid: String,
    pub role_id: i64,
    pub created_ts: i64,
    pub updated_ts: i64,
    pub session_token: String,  // New field - for resolver to set cookie
}
```



### Cookie Setting Pattern
Use identical pattern from `authLogin` resolver:
```rust
let cookie_value = format!(
    "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
    SESSION_COOKIE_NAME,
    register_result.session_token,
    cookie_domain,
    config.api_path,
    if insecure_cookie { "" } else { "; Secure" },
    config.session_ttl_seconds
);
let _ = ctx.append_http_header("set-cookie", cookie_value);
```

## Testing Strategy

1. **Unit Tests**: Test new AuthService method with session creation
2. **Resolver Tests**: Test cookie setting in GraphQL resolver
3. **Integration Tests**: Test full flow with HTTP headers
4. **Backward Compatibility**: Ensure existing register() method still works

## Security Considerations

- Session creation should only happen after successful user creation
- Use same session secret and security settings as login
- Ensure password validation happens before session creation
- Log session creation events appropriately (without sensitive data)