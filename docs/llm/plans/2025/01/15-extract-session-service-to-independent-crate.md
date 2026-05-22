# Extract SessionService Components to Independent Crate

**Date:** 2025-01-15@10:30  
**Task:** Extract authentication-related components from SessionService into a fully independent crate named `dp-auth-session-service`

## Overview

This plan outlines the extraction of token encoding/decoding functionality from the existing SessionService into a new independent crate. The extraction will be done in two major phases to ensure proper testing and integration.

## Current State Analysis

### Components to Extract

From `src/services/session_service.rs`:

**Methods:**
- `encode_token(payload: &SessionPayload, secret: &[u8]) -> Result<String, SessionError>`
- `decode_token(token: &str, secret: &[u8]) -> Result<SessionPayload, SessionError>`
- `create_payload(user_id: i64) -> SessionPayload` (helper method)

**Structs:**
- `SessionPayload` → `DpAuthSessionPayload`

**Errors (subset):**
- `SessionError::EncodingError(String)` → `DpAuthSessionError::EncodingError(String)`
- `SessionError::DecodingError(String)` → `DpAuthSessionError::DecodingError(String)`
- `SessionError::TokenExpired` → `DpAuthSessionError::TokenExpired`
- `SessionError::InvalidToken(String)` → `DpAuthSessionError::InvalidToken(String)`
- `SessionError::JsonError(String)` → `DpAuthSessionError::JsonError(String)`

### Components to Remain in Original SessionService

**Methods:**
- `create_session(pool: &SqlitePool, username: &str, password: &str, secret: &[u8]) -> Result<String, SessionError>`

**Errors (subset):**
- `SessionError::AuthenticationError(String)`
- `SessionError::DatabaseError(String)`
- `SessionError::PasswordVerificationError(String)`

### Current Dependencies

The components to be extracted currently depend on:
- `aes-gcm` (encryption)
- `base64` (encoding)
- `serde` (serialization)
- `chrono` (timestamp handling)
- `rand` (nonce generation)

## Phase 1: Create Independent Crate

**Directory:** `dp-auth-session-service-component/`

### 1.1 Crate Structure

```
dp-auth-session-service-component/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── payload.rs
│   ├── service.rs
│   └── error.rs
└── tests/
    └── integration_tests.rs
```

### 1.2 New Components

#### `DpAuthSessionService` (in `src/service.rs`)
```rust
pub struct DpAuthSessionService;

impl DpAuthSessionService {
    pub fn encode_token(payload: &DpAuthSessionPayload, secret: &[u8]) -> Result<String, DpAuthSessionError>
    pub fn decode_token(token: &str, secret: &[u8]) -> Result<DpAuthSessionPayload, DpAuthSessionError>
    pub fn create_payload(user_id: i64) -> DpAuthSessionPayload
}
```

#### `DpAuthSessionPayload` (in `src/payload.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DpAuthSessionPayload {
    pub sub: i64,
    pub iat: i64,
    pub exp: i64,
}
```

#### `DpAuthSessionError` (in `src/error.rs`)
```rust
#[derive(Debug)]
pub enum DpAuthSessionError {
    EncodingError(String),
    DecodingError(String),
    TokenExpired,
    InvalidToken(String),
    JsonError(String),
}
```

### 1.3 Dependencies (Cargo.toml)
```toml
[package]
name = "dp-auth-session-service"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
aes-gcm = "0.10"
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
rand = "0.8"
```

### 1.4 Testing Strategy

- **Unit tests:** Test each method individually with various inputs
- **Integration tests:** Test complete encode/decode workflows
- **Error handling tests:** Verify all error conditions
- **Edge case tests:** Expired tokens, invalid formats, wrong secrets

### 1.5 Documentation

- Complete Rust doc comments for all public APIs
- Usage examples in documentation
- README.md with basic usage instructions

## Phase 2: Integration with Main Crate

**Note:** Phase 2 will only begin after Phase 1 is complete and the new crate is published to a GitHub repository.

### 2.1 Update Main Crate Dependencies

Add to main `Cargo.toml`:
```toml
dp-auth-session-service = { git = "https://github.com/user/dp-auth-session-service", version = "0.1.0" }
```

### 2.2 Update Imports and Usage

#### In `src/services/session_service.rs`:
- Remove extracted methods: `encode_token`, `decode_token`, `create_payload`
- Remove extracted error variants from `SessionError`
- Remove `SessionPayload` struct
- Update `create_session` method to use new crate:
  ```rust
  use dp_auth_session_service::{DpAuthSessionService, DpAuthSessionPayload};
  
  // In create_session method:
  let payload = DpAuthSessionService::create_payload(user.id);
  let token = DpAuthSessionService::encode_token(&payload, secret)?;
  ```

#### In `src/middleware/session.rs`:
- Update imports:
  ```rust
  use dp_auth_session_service::{DpAuthSessionService, DpAuthSessionPayload};
  ```
- Update `extract_and_validate_session_sync`:
  ```rust
  if let Ok(payload) = DpAuthSessionService::decode_token(&token, secret) {
      return SessionContext::new(Some(payload));
  }
  ```

#### In `src/graphql/types/session_payload.rs`:
- Update import and From implementation:
  ```rust
  use dp_auth_session_service::DpAuthSessionPayload as ServiceSessionPayload;
  ```

#### In `src/services/mod.rs`:
- Remove `SessionPayload` from exports
- Add re-export: `pub use dp_auth_session_service::DpAuthSessionPayload as SessionPayload;`

### 2.3 Error Handling Updates

#### In `src/graphql/mutations/create_session.rs`:
- Update error mapping function to handle both old and new error types
- Map `DpAuthSessionError::EncodingError` to "Internal server error"

### 2.4 Testing Updates

- Update all tests that use the extracted components
- Ensure integration tests still pass
- Update test imports to use new crate components

### 2.5 Files to Modify in Phase 2

1. `src/services/session_service.rs` - Remove extracted methods and types
2. `src/services/mod.rs` - Update exports
3. `src/middleware/session.rs` - Update imports and usage
4. `src/graphql/types/session_payload.rs` - Update imports
5. `src/graphql/mutations/create_session.rs` - Update error handling
6. `src/graphql/queries/get_current_session.rs` - Update imports if needed
7. `Cargo.toml` - Add new dependency
8. All test files that reference the extracted components

## Success Criteria

### Phase 1 Complete When:
- [ ] New crate compiles without errors
- [ ] All unit tests pass (>95% code coverage)
- [ ] Integration tests demonstrate full encode/decode workflow
- [ ] Documentation is complete with examples
- [ ] Crate is ready for independent use

### Phase 2 Complete When:
- [ ] Main crate compiles with new dependency
- [ ] All existing tests pass
- [ ] No functionality is lost
- [ ] Performance is maintained
- [ ] Clean separation of concerns achieved

## Risk Mitigation

1. **Dependency conflicts:** Use same versions as main crate where possible
2. **API compatibility:** Maintain identical method signatures during extraction
3. **Test coverage:** Comprehensive testing before integration
4. **Rollback plan:** Keep original code until Phase 2 is verified working

## Timeline Estimate

- **Phase 1:** 2-3 hours (crate creation, testing, documentation)
- **Phase 2:** 1-2 hours (integration, testing, cleanup)
- **Total:** 3-5 hours

---

**Next Steps:** Begin Phase 1 by creating the new crate structure and implementing the extracted components.