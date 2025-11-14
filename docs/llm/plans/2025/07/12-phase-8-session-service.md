# Phase 8: Session Service - Token Management

**Date**: 2025-07-12  
**Phase**: 8  
**Status**: Planning  

## Overview

Implement `SessionService` for managing session tokens using a custom encrypted token format (not JWT). The service will handle token encoding/decoding with a secret key and manage session payloads containing user information.

## Requirements Analysis

From the README, Phase 8 requires:

1. **Custom Token Format**: Use encrypted tokens instead of JWT to avoid exposing payload structure
2. **Session Payload Structure**: JSON-serializable with fields:
   - `sub` (subject - user id)
   - `iat` (issued at timestamp) 
   - `exp` (expiration timestamp)
3. **Secret Key Management**: Use environment variable `DP_AUTH_SECRET_KEY`
4. **Core Methods**:
   - `encode_token` - Create session token from payload
   - `decode_token` - Decode token and retrieve payload
5. **Error Handling**: Determine behavior when secret key is not set
6. **Testing**: Unit tests with full coverage
7. **Documentation**: Rust doc comments

## Technical Design

### Dependencies to Add

Add to `Cargo.toml`:
```toml
# For encryption/decryption
aes-gcm = "0.10"
base64 = "0.22"
```

### Session Payload Structure

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionPayload {
  /// Subject - user ID
  pub sub: i64,
  /// Issued at timestamp (seconds since epoch)
  pub iat: i64,
  /// Expiration timestamp (seconds since epoch)
  pub exp: i64,
}
```

### Error Types

```rust
#[derive(Debug)]
pub enum SessionError {
  /// Secret key not configured
  SecretKeyNotSet,
  /// Invalid secret key format
  InvalidSecretKey(String),
  /// Token encoding failed
  EncodingError(String),
  /// Token decoding failed
  DecodingError(String),
  /// Token has expired
  TokenExpired,
  /// Invalid token format
  InvalidToken(String),
  /// JSON serialization/deserialization error
  JsonError(String),
}
```

### SessionService Implementation

```rust
pub struct SessionService;

impl SessionService {
  /// Default token expiration time (3 days in seconds)
  const DEFAULT_EXPIRATION_SECONDS: i64 = 3 * 24 * 60 * 60;

  /// Encode a session payload into an encrypted token
  pub fn encode_token(payload: &SessionPayload) -> Result<String, SessionError>;

  /// Decode an encrypted token and retrieve the session payload
  pub fn decode_token(token: &str) -> Result<SessionPayload, SessionError>;

  /// Create a new session payload for a user
  pub fn create_payload(user_id: i64) -> SessionPayload;

  /// Get the secret key from environment variable
  fn get_secret_key() -> Result<Vec<u8>, SessionError>;

  /// Encrypt data using AES-GCM
  fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>, SessionError>;

  /// Decrypt data using AES-GCM
  fn decrypt(encrypted_data: &[u8], key: &[u8]) -> Result<Vec<u8>, SessionError>;
}
```

### `encode_token` Implementation Example

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce, AeadCore, AeadInPlace, KeyInit};
use base64::{Engine as _, engine::general_purpose};

impl SessionService {
  /// Encode a session payload into an encrypted token
  pub fn encode_token(payload: &SessionPayload) -> Result<String, SessionError> {
    // Get the secret key from environment
    let key_bytes = Self::get_secret_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Serialize payload to JSON
    let json_data = serde_json::to_vec(payload)
      .map_err(|e| SessionError::JsonError(e.to_string()))?;

    // Generate a random nonce
    let nonce = Aes256Gcm::generate_nonce(&mut rand::rngs::OsRng);
    
    // Encrypt the JSON data
    let mut buffer = json_data;
    cipher.encrypt_in_place(&nonce, b"", &mut buffer)
      .map_err(|e| SessionError::EncodingError(e.to_string()))?;

    // Combine nonce + encrypted data
    let mut result = nonce.to_vec();
    result.extend_from_slice(&buffer);

    // Base64 encode the result
    Ok(general_purpose::STANDARD.encode(result))
  }
}
```

### `decode_token` Implementation Example

```rust
impl SessionService {
  /// Decode an encrypted token and retrieve the session payload
  pub fn decode_token(token: &str) -> Result<SessionPayload, SessionError> {
    // Get the secret key from environment
    let key_bytes = Self::get_secret_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Base64 decode the token
    let encrypted_data = general_purpose::STANDARD.decode(token)
      .map_err(|e| SessionError::InvalidToken(format!("Base64 decode error: {}", e)))?;

    // Check minimum length (nonce + some encrypted data)
    if encrypted_data.len() < 12 {
      return Err(SessionError::InvalidToken("Token too short".to_string()));
    }

    // Split nonce and encrypted payload
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt the data
    let mut buffer = ciphertext.to_vec();
    cipher.decrypt_in_place(nonce, b"", &mut buffer)
      .map_err(|e| SessionError::DecodingError(format!("Decryption failed: {}", e)))?;

    // Deserialize JSON to payload
    let payload: SessionPayload = serde_json::from_slice(&buffer)
      .map_err(|e| SessionError::JsonError(e.to_string()))?;

    // Check if token has expired
    let current_time = chrono::Utc::now().timestamp();
    if payload.exp < current_time {
      return Err(SessionError::TokenExpired);
    }

    Ok(payload)
  }
}
```

### `create_payload` Implementation Example

```rust
impl SessionService {
  /// Create a new session payload for a user
  pub fn create_payload(user_id: i64) -> SessionPayload {
    let current_time = chrono::Utc::now().timestamp();
    
    SessionPayload {
      sub: user_id,
      iat: current_time,
      exp: current_time + Self::DEFAULT_EXPIRATION_SECONDS,
    }
  }
}
```

### `get_secret_key` Implementation Example

```rust
impl SessionService {
  /// Get the secret key from environment variable
  fn get_secret_key() -> Result<Vec<u8>, SessionError> {
    let key_str = std::env::var("DP_AUTH_SECRET_KEY")
      .map_err(|_| SessionError::SecretKeyNotSet)?;

    // Decode base64 key
    let key_bytes = general_purpose::STANDARD.decode(&key_str)
      .map_err(|e| SessionError::InvalidSecretKey(format!("Base64 decode error: {}", e)))?;

    // Ensure key is at least 32 bytes for AES-256
    if key_bytes.len() < 32 {
      return Err(SessionError::InvalidSecretKey(
        format!("Key too short: {} bytes, need at least 32", key_bytes.len())
      ));
    }

    // Use exactly 32 bytes for AES-256
    Ok(key_bytes[..32].to_vec())
  }
}
```

### Token Format

The token will be base64-encoded encrypted JSON:
1. Serialize `SessionPayload` to JSON
2. Encrypt JSON bytes using AES-GCM with secret key
3. Base64 encode the encrypted bytes
4. Return as string token

### Secret Key Handling

- Read from `DP_AUTH_SECRET_KEY` environment variable
- If not set, return `SessionError::SecretKeyNotSet`
- Key should be at least 32 bytes for AES-256
- If key is too short, return `SessionError::InvalidSecretKey`

### Token Validation

During decoding:
1. Base64 decode the token
2. Decrypt using secret key
3. Deserialize JSON to `SessionPayload`
4. Check if token has expired (`exp` < current timestamp)
5. Return payload or appropriate error

## Files to Create/Modify

### New Files

1. **`src/services/session_service.rs`**
   - Complete `SessionService` implementation
   - All error types and payload structures
   - Full documentation with examples

### Modified Files

1. **`src/services/mod.rs`**
   - Add `session_service` module
   - Export `SessionService` and `SessionError`

2. **`Cargo.toml`**
   - Add `aes-gcm` and `base64` dependencies

## Testing Strategy

### Unit Tests (`src/services/session_service.rs`)

1. **`test_create_payload`**
   - Verify payload structure and timestamps
   - Check expiration is set correctly

2. **`test_encode_decode_roundtrip`**
   - Encode payload to token, then decode back
   - Verify payload matches original

3. **`test_decode_expired_token`**
   - Create payload with past expiration
   - Verify `TokenExpired` error is returned

4. **`test_decode_invalid_token`**
   - Test various invalid token formats
   - Verify appropriate errors are returned

5. **`test_missing_secret_key`**
   - Test behavior when `DP_AUTH_SECRET_KEY` is not set
   - Verify `SecretKeyNotSet` error

6. **`test_invalid_secret_key`**
   - Test with keys that are too short
   - Verify `InvalidSecretKey` error

7. **`test_encode_decode_with_different_keys`**
   - Encode with one key, try to decode with another
   - Verify decoding fails appropriately

### Integration Tests

Integration tests will be implemented in later phases when the SessionService is actually used by GraphQL resolvers and middleware.

## Environment Configuration

### Secret Key Generation

Generate secure secret keys using a simple one-liner terminal command:

**Generate and display a secret key:**
```bash
openssl rand -base64 32
```

**Generate and set environment variable directly:**
```bash
export DP_AUTH_SECRET_KEY=$(openssl rand -base64 32)
```

### Development
```bash
# Generate and set a secret key for development
export DP_AUTH_SECRET_KEY=$(openssl rand -base64 32)

# Or add to your .env file (with proper newline)
echo "" >> .env
echo "# Session service configuration" >> .env
echo "DP_AUTH_SECRET_KEY=$(openssl rand -base64 32)" >> .env
```

### Production
- Use the same `openssl rand -base64 32` command to generate production keys
- Store securely in environment variables or secrets management
- Never commit secret keys to version control
- Consider key rotation strategies for future phases

## Security Considerations

1. **Key Management**: Secret key must be kept secure and not logged
2. **Token Expiration**: Implement reasonable default expiration (3 days)
3. **Encryption**: Use AES-GCM for authenticated encryption
4. **Error Messages**: Don't leak sensitive information in error messages
5. **Timing Attacks**: Consider constant-time operations where applicable

## Code Examples

### Basic Usage
```rust
// Create a session payload
let payload = SessionService::create_payload(user_id);

// Encode to token
let token = SessionService::encode_token(&payload)?;

// Decode token back to payload
let decoded_payload = SessionService::decode_token(&token)?;
```

### Error Handling
```rust
match SessionService::decode_token(&token) {
  Ok(payload) => {
    // Use valid payload
  },
  Err(SessionError::TokenExpired) => {
    // Handle expired token
  },
  Err(SessionError::InvalidToken(_)) => {
    // Handle invalid token
  },
  Err(e) => {
    // Handle other errors
  }
}
```

## Implementation Steps

1. **Add Dependencies**: Update `Cargo.toml` with required crates
2. **Environment Setup**: Update `.env` file with generated secret key for development
3. **Create Session Types**: Implement `SessionPayload` and `SessionError`
4. **Implement Core Service**: Create `SessionService` with encode/decode methods
5. **Add Secret Key Management**: Environment variable handling
6. **Implement Encryption**: AES-GCM encryption/decryption
7. **Add Module Exports**: Update `mod.rs` files
8. **Write Unit Tests**: Comprehensive test coverage
9. **Documentation**: Add Rust doc comments

## Success Criteria

- [ ] `SessionService` successfully encodes payloads to encrypted tokens
- [ ] `SessionService` successfully decodes tokens back to payloads
- [ ] Token expiration is properly validated
- [ ] Secret key management works correctly
- [ ] All error cases are handled appropriately
- [ ] Unit tests achieve 100% coverage
- [ ] Documentation is complete and clear
- [ ] Integration tests pass
- [ ] No sensitive data is logged or exposed

## Future Considerations

- Token revocation (Phase 12+ with `user_sessions` table)
- Key rotation strategies
- Token refresh mechanisms
- Rate limiting for token operations
- Audit logging for security events