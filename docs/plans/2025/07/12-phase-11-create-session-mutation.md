# Phase 11: `createSession` Mutation (Sign-in)

**Date**: 2025-07-12@08:30

## Overview

This phase implements the `createSession` GraphQL mutation that allows users to sign in by providing their username and password. The mutation will authenticate the user, create a session token, and set a cookie in the response for browser-based clients.

## Requirements from README

- [ ] Implement `createSession` GraphQL mutation
  - [ ] Accepts username and password as input
  - [ ] Calls `SessionService::create_session`
    - [ ] On success, returns the session token and sets the cookie with the token in the response
    - [ ] On failure, returns a user-friendly error message (not the internal error message), e.g., "Invalid credentials" for any username or password error, or "Internal server error" for unexpected errors
  - [ ] Make a decision if errors should return 2XX or 4XX status codes, as it impacts the client-side error handling
- [ ] Unit tests for the `createSession` mutation, ensuring it calls the `SessionService::create_session` method with the correct parameters
- [ ] Integration tests for the `createSession` mutation, ensuring it returns a session token and sets the cookie in the response

## Current State Analysis

### Existing Infrastructure
- ✅ `SessionService::create_session` method is fully implemented and tested
- ✅ Session middleware supports cookie extraction with `SESSION_COOKIE_NAME = "DpAuthSession"`
- ✅ GraphQL mutation structure is established with `CreateUserMutation` as a reference
- ✅ Error handling patterns are established in existing mutations

### SessionService::create_session Method
The method is already implemented with the following signature:
```rust
pub async fn create_session(
  pool: &SqlitePool,
  username: &str,
  password: &str,
) -> Result<String, SessionError>
```

It returns specific error types:
- `AuthenticationError`: For user validation issues (blank username/password, user not found, wrong password)
- `DatabaseError`: For database operation failures
- `PasswordVerificationError`: For password hashing issues
- Token encoding errors from `encode_token`

## Design Decisions

### 1. HTTP Status Codes for Authentication Errors

**Decision**: Use 2XX status codes for GraphQL responses, even for authentication failures.

**Rationale**:
- GraphQL convention is to return 200 OK for successful transport, with errors in the GraphQL response body
- This allows clients to distinguish between transport errors (network, server down) and business logic errors (invalid credentials)
- Consistent with existing `createUser` mutation pattern
- Client-side GraphQL libraries expect this pattern

### 2. Error Message Mapping

**Decision**: Map internal `SessionError` types to user-friendly messages:
- `AuthenticationError` → "Invalid credentials"
- `DatabaseError` → "Internal server error"
- `PasswordVerificationError` → "Internal server error"
- Token encoding errors → "Internal server error"

**Rationale**:
- Security: Don't expose internal error details that could aid attackers
- User experience: Provide clear, actionable feedback
- Consistency: All authentication failures appear the same to prevent username enumeration

### 3. Cookie Configuration

**Domain Architecture Context**:
The system uses a subdomain-based architecture:
- Main domain: `mysite.com`
- Auth API: `auth.api.mysite.com` (this service)
- Other APIs: `<project>.api.mysite.com` (will read the session cookie)
- Frontend projects: `<project>.mysite.com`
- Data services: `data.mysite.com` (S3 buckets, should NOT receive cookies)

**Decision**: Set secure, HTTP-only cookie with restricted domain scope for maximum security:
- Name: `DpAuthSession` (using existing `SESSION_COOKIE_NAME` constant)
- HttpOnly: `true` (prevent XSS access)
- Secure: Configurable via `DP_AUTH_INSECURE_COOKIE` environment variable
  - `true` (default) - Secure cookies for production (HTTPS only)
  - `false` if `DP_AUTH_INSECURE_COOKIE` is set - Allow HTTP for local development
- SameSite: `Strict` (appropriate for this subdomain architecture)
- Domain: Read from `DP_AUTH_COOKIE_DOMAIN` environment variable (e.g., `.api.mysite.com`)
- Max-Age: Match token expiration (3 days)

**Cookie Domain Strategy - `.api.mysite.com`**:
Using `.api.mysite.com` instead of `.mysite.com` provides superior security:

**Security Benefits**:
1. **Principle of Least Privilege**: Cookies only sent to services that need them (APIs)
2. **XSS Protection**: Frontends cannot access cookies via JavaScript - browser handles automatically
3. **Data Leakage Prevention**: Session cookies never sent to:
   - `data.mysite.com` (S3 buckets)
   - `cdn.mysite.com` (CDN services)
   - `assets.mysite.com` (static file servers)
   - Any future non-API subdomains

**Functional Benefits**:
1. **Automatic Cookie Handling**: Browser includes cookies in XHR/fetch from `<project>.mysite.com` to `<project>.api.mysite.com`
2. **API-to-API Communication**: All APIs can authenticate each other using cookies
3. **Cleaner Logs**: Session data doesn't pollute S3 access logs, CDN logs, or analytics
4. **Future-Proof**: New services outside `.api` subdomain won't accidentally receive session cookies

**SameSite=Strict Analysis**:
`SameSite=Strict` works perfectly because:
1. **Same-Site Requests**: All API calls from frontends (`<project>.mysite.com`) to APIs (`<project>.api.mysite.com`) are same-site requests since they share the same eTLD+1 (`mysite.com`)
2. **Cross-Site Protection**: Prevents the cookie from being sent in cross-site requests from external domains, providing strong CSRF protection
3. **Subdomain Isolation**: The cookie domain `.api.mysite.com` ensures cookies only flow to API services while maintaining strict same-site enforcement
4. **No External Redirects**: Since authentication happens within the same domain ecosystem, there are no legitimate cross-site scenarios that would require `Lax`

This provides maximum security while maintaining full functionality within the subdomain architecture.

## Implementation Plan

### 1. Create Input/Output Types

**File**: `src/graphql/mutations/create_session.rs`

```rust
/// Input type for creating a new session (sign-in)
#[derive(InputObject)]
pub struct CreateSessionInput {
  /// Username for authentication
  pub username: String,
  /// Password for authentication
  pub password: String,
}

/// GraphQL output type for session creation response
#[derive(SimpleObject)]
pub struct CreateSessionResponse {
  /// The session token for API authentication
  pub token: String,
  /// Success message
  pub message: String,
}
```

### 2. Implement Mutation Resolver

**File**: `src/graphql/mutations/create_session.rs`

```rust
/// GraphQL mutation for creating user sessions (sign-in)
pub struct CreateSessionMutation;

#[Object]
impl CreateSessionMutation {
  /// Create a new session by authenticating user credentials
  #[instrument(skip(self, ctx, input), fields(username = %input.username))]
  async fn create_session(
    &self,
    ctx: &Context<'_>,
    input: CreateSessionInput,
  ) -> Result<CreateSessionResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    
    match SessionService::create_session(pool, &input.username, &input.password).await {
      Ok(token) => {
        // Set cookie in response
        if let Some(response_headers) = ctx.data_opt::<ResponseHeaders>() {
          set_session_cookie(response_headers, &token);
        }
        
        Ok(CreateSessionResponse {
          token,
          message: "Authentication successful".to_string(),
        })
      }
      Err(session_error) => {
        let user_message = map_session_error_to_user_message(&session_error);
        Err(async_graphql::Error::new(user_message))
      }
    }
  }
}

fn map_session_error_to_user_message(error: &SessionError) -> &'static str {
  match error {
    SessionError::AuthenticationError(_) => "Invalid credentials",
    SessionError::DatabaseError(_) => "Internal server error",
    SessionError::PasswordVerificationError(_) => "Internal server error",
    SessionError::EncodingError(_) => "Internal server error",
    _ => "Internal server error",
  }
}
```

### 3. Cookie Setting Implementation

**Challenge**: GraphQL resolvers don't have direct access to HTTP response headers.

**Solution**: Use Axum's response extensions or a custom response wrapper.

**Approach**: Modify the GraphQL handler to support cookie setting:

```rust
// In src/handlers/graphql.rs - add response header handling
pub struct ResponseHeaders {
  pub headers: Arc<Mutex<HeaderMap>>,
}

// Cookie setting utility function
fn set_session_cookie(response_headers: &ResponseHeaders, token: &str) {
  let cookie_domain = std::env::var("DP_AUTH_COOKIE_DOMAIN")
    .unwrap_or_else(|_| ".api.localhost".to_string()); // Default for development
  
  let is_insecure = std::env::var("DP_AUTH_INSECURE_COOKIE").is_ok();
  let secure_flag = if is_insecure { "" } else { "; Secure" };
  
  let cookie_value = format!(
    "{}={}; Domain={}; Path=/; HttpOnly; SameSite=Strict{}; Max-Age={}",
    SESSION_COOKIE_NAME,
    token,
    cookie_domain,
    secure_flag,
    3 * 24 * 60 * 60 // 3 days in seconds
  );
  
  if let Ok(mut headers) = response_headers.headers.lock() {
    headers.insert("Set-Cookie", cookie_value.parse().unwrap());
  }
}

// Modify graphql_post_handler to extract and apply response headers
```

### 4. Update Mutation Root

**File**: `src/graphql/mutations/mod.rs`
```rust
pub mod create_session;
pub mod create_user;

pub use create_session::CreateSessionMutation;
pub use create_user::CreateUserMutation;
```

**File**: `src/graphql/mutation.rs`
```rust
#[derive(MergedObject, Default)]
pub struct Mutation(CreateUserMutation, CreateSessionMutation);
```

### 5. Unit Tests

**File**: `src/graphql/mutations/create_session.rs`

Test cases:
- `test_create_session_calls_service_with_correct_parameters()`
- `test_create_session_maps_authentication_error()`
- `test_create_session_maps_database_error()`
- `test_create_session_maps_encoding_error()`

### 6. Integration Tests

**File**: `tests/integration_tests.rs`

Test cases:
- `test_create_session_mutation_success()`
- `test_create_session_mutation_invalid_credentials()`
- `test_create_session_mutation_sets_cookie()`
- `test_create_session_mutation_with_missing_user()`

## Files to Create/Modify

### New Files
- `src/graphql/mutations/create_session.rs` - Main mutation implementation

### Modified Files
- `src/graphql/mutations/mod.rs` - Export new mutation
- `src/graphql/mutation.rs` - Add to merged object
- `src/handlers/graphql.rs` - Add cookie setting support
- `tests/integration_tests.rs` - Add integration tests

## Testing Strategy

### Unit Tests
- Mock `SessionService::create_session` calls
- Verify correct parameter passing
- Test error mapping logic
- Verify cookie setting calls

### Integration Tests
- End-to-end GraphQL mutation calls
- Database integration with real user creation
- Cookie verification in HTTP responses
- Error response validation

## Security Considerations

1. **Error Message Sanitization**: Never expose internal error details
2. **Rate Limiting**: Consider implementing in future phases
3. **Cookie Security**: HttpOnly, Secure, SameSite attributes
4. **Logging**: Log authentication attempts without passwords
5. **Timing Attacks**: `SessionService` already handles this with consistent password verification

## Environment Configuration

### Development Environment Setup
Add the following to your local `.env` file to enable development mode:

```env
# Allow insecure cookies for local development (HTTP instead of HTTPS)
DP_AUTH_INSECURE_COOKIE=true

# Cookie domain for local development
DP_AUTH_COOKIE_DOMAIN=.api.dp-auth.localhost
```

**Important**: The `.env` file should be updated to include these development-specific settings. The `DP_AUTH_INSECURE_COOKIE` variable is required for local development since most local setups run on HTTP rather than HTTPS.

### Production Environment
In production, these variables should be configured as:
```env
# DP_AUTH_INSECURE_COOKIE should NOT be set (defaults to secure cookies)
DP_AUTH_COOKIE_DOMAIN=.api.yourdomain.com
```

## Dependencies

- No new external dependencies required
- Relies on existing `SessionService::create_session` implementation
- Uses established GraphQL patterns from `CreateUserMutation`

## Success Criteria

- [ ] `createSession` mutation accepts username/password and returns token
- [ ] Successful authentication sets `DpAuthSession` cookie
- [ ] Authentication errors return user-friendly messages
- [ ] Unit tests achieve 100% coverage of mutation logic
- [ ] Integration tests verify end-to-end functionality
- [ ] Cookie security attributes are properly configured
- [ ] Error responses follow GraphQL conventions (2XX with error in body)

## Future Considerations

- Session invalidation/logout mutation
- Remember me functionality with longer-lived tokens
- Multi-factor authentication support
- Session management dashboard
- Token refresh mechanism