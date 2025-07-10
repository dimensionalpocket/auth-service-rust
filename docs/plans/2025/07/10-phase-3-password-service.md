# Phase 3: Password Service Implementation

**Date**: 2025-07-10  
**Phase**: 3 of 4  
**Goal**: Implement secure password hashing and verification service using Argon2

## Overview

This phase implements the `PasswordService` for secure user password management. The service will provide methods for generating password hashes with salt and verifying passwords against existing hashes. This follows the project's design pattern of static service methods with comprehensive unit testing.

## Requirements Analysis

Based on the README specifications:
- Use `argon2` for password hashing
- Generate 16-byte salt using `rand::rngs::OsRng`
- Implement `generate` method to create password hashes (includes salt generation)
- Implement `verify` method to check passwords against hashes
- Follow existing service patterns (static methods, comprehensive tests, Rust doc comments)
- Store service in `src/services/password_service.rs`

## Tasks Breakdown

### 1. Dependencies Setup

Add password-related dependencies to `Cargo.toml`:
```toml
[dependencies]
# Existing dependencies...
argon2 = "0.5"
rand = "0.8"

[dev-dependencies]
# Existing dev dependencies...
# (no additional dev dependencies needed)
```

### 2. PasswordService Implementation

#### Core Structure
- Create `src/services/password_service.rs`
- Implement static struct `PasswordService` following existing patterns
- Use Argon2id variant (recommended for password hashing)
- Configure appropriate Argon2 parameters for security vs performance balance

#### Methods to Implement

**`generate(password: &str) -> Result<String, PasswordError>`**
- Generate 16-byte salt using `rand::rngs::OsRng`
- Hash password with Argon2id algorithm
- Return encoded hash string (includes algorithm, parameters, salt, and hash)
- Handle errors appropriately

**`verify(password: &str, hash: &str) -> Result<bool, PasswordError>`**
- Parse the encoded hash string
- Extract salt and parameters from hash
- Hash the provided password with same salt and parameters
- Compare hashes securely (constant-time comparison)
- Return boolean result

#### Error Handling
- Define custom `PasswordError` enum for different error types:
  - `HashingError` - Issues during hash generation
  - `VerificationError` - Issues during verification
  - `InvalidHash` - Malformed hash strings
- Implement proper error conversion from Argon2 errors

### 3. Module Integration

Update `src/services/mod.rs`:
```rust
pub mod server_service;
pub mod password_service;

pub use server_service::ServerService;
pub use password_service::{PasswordService, PasswordError};
```

### 4. Comprehensive Testing

#### Unit Tests (`src/services/password_service.rs`)

**Basic Functionality Tests:**
- `test_generate_creates_valid_hash` - Verify hash generation works
- `test_generate_different_salts` - Ensure different salts for same password
- `test_verify_correct_password` - Verify correct password validation
- `test_verify_incorrect_password` - Verify incorrect password rejection
- `test_verify_invalid_hash_format` - Handle malformed hash strings

**Security Tests:**
- `test_generate_uses_different_salts` - Verify salt uniqueness
- `test_hash_format_contains_expected_components` - Verify hash structure
- `test_verify_timing_consistency` - Basic timing attack resistance check

**Edge Cases:**
- `test_empty_password_handling` - Handle empty passwords
- `test_very_long_password_handling` - Handle long passwords (within reason)
- `test_unicode_password_handling` - Handle Unicode characters
- `test_special_characters_in_password` - Handle special characters

**Error Handling Tests:**
- `test_verify_with_invalid_hash_returns_error` - Invalid hash format
- `test_verify_with_empty_hash_returns_error` - Empty hash string

### 5. Documentation

#### Rust Doc Comments
Following the pattern from `ServerService`, add comprehensive documentation:

```rust
/// Service for secure password hashing and verification using Argon2
pub struct PasswordService;

impl PasswordService {
  /// Generates a secure password hash using Argon2id with a random salt
  ///
  /// This method creates a new 16-byte salt using a cryptographically secure
  /// random number generator and hashes the provided password using Argon2id.
  ///
  /// # Arguments
  ///
  /// * `password` - The plaintext password to hash
  ///
  /// # Returns
  ///
  /// * `Ok(String)` - The encoded hash string containing algorithm, parameters, salt, and hash
  /// * `Err(PasswordError)` - If hashing fails
  ///
  /// # Examples
  ///
  /// ```
  /// use dp_auth_service::services::PasswordService;
  ///
  /// let hash = PasswordService::generate("my_secure_password").unwrap();
  /// assert!(!hash.is_empty());
  /// ```
  pub fn generate(password: &str) -> Result<String, PasswordError> {
    // Implementation
  }

  /// Verifies a password against a previously generated hash
  ///
  /// This method extracts the salt and parameters from the encoded hash
  /// and verifies the provided password against it using constant-time comparison.
  ///
  /// # Arguments
  ///
  /// * `password` - The plaintext password to verify
  /// * `hash` - The encoded hash string to verify against
  ///
  /// # Returns
  ///
  /// * `Ok(true)` - If the password matches the hash
  /// * `Ok(false)` - If the password does not match the hash
  /// * `Err(PasswordError)` - If verification fails due to invalid hash format
  ///
  /// # Examples
  ///
  /// ```
  /// use dp_auth_service::services::PasswordService;
  ///
  /// let hash = PasswordService::generate("my_password").unwrap();
  /// let is_valid = PasswordService::verify("my_password", &hash).unwrap();
  /// assert!(is_valid);
  /// ```
  pub fn verify(password: &str, hash: &str) -> Result<bool, PasswordError> {
    // Implementation
  }
}
```

## Implementation Details

### Argon2 Configuration
- **Variant**: Argon2id (recommended for password hashing)
- **Memory Cost**: 4096 KB (4 MB) - reasonable for production with multiple concurrent users
- **Time Cost**: 3 iterations - good security/performance balance
- **Parallelism**: 1 thread - simplicity for this phase
- **Salt Length**: 16 bytes as specified
- **Hash Length**: 32 bytes (standard)

#### Memory Usage Pattern
- **Temporary allocation**: The 4MB is allocated only during the actual hashing operation
- **Per-operation**: Each password hash/verify operation uses its own 4MB allocation
- **Concurrent operations**: Multiple simultaneous operations multiply memory usage (e.g., 10 concurrent logins = 40MB temporarily)
- **Immediate cleanup**: Memory is freed as soon as each individual operation completes
- **No persistent storage**: No memory is kept between operations

#### Expected Performance
- **Estimated timing**: With these parameters, each operation should take approximately 50-200ms on modern hardware
- **Hardware dependency**: Actual timing varies significantly based on CPU and memory speed
- **Consistent cost**: Both `generate` and `verify` operations have similar performance characteristics
- **Production consideration**: This timing is acceptable for authentication flows but may need tuning based on specific requirements

#### Parameter Evolution and Backward Compatibility
- **Self-describing hashes**: Argon2 hashes embed their parameters in the hash string (e.g., `$argon2id$v=19$m=4096,t=3,p=1$...`)
- **Automatic compatibility**: When verifying, Argon2 extracts and uses the original parameters from the hash
- **Parameter changes**: Changing default parameters only affects new hashes; existing hashes continue to verify correctly
- **Migration strategy**: If upgrading parameters is desired:
  - Detect hashes with old parameters during successful login
  - Re-hash with new parameters when user authenticates
  - Gradually migrate database over time without breaking existing users
- **Production benefit**: No service disruption when tuning Argon2 parameters for performance optimization

### Security Considerations
- Use `OsRng` for cryptographically secure salt generation
- Ensure constant-time comparison in verification
- Handle errors without leaking timing information
- Use appropriate Argon2 parameters for current security standards

## Files to be Created/Modified

### New Files
1. **`src/services/password_service.rs`** - Main service implementation
   ```rust
   use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
   use argon2::password_hash::{rand_core::OsRng, SaltString};
   use std::fmt;

   #[derive(Debug)]
   pub enum PasswordError {
     HashingError(String),
     VerificationError(String),
     InvalidHash(String),
   }

   impl fmt::Display for PasswordError {
     // Implementation
   }

   impl std::error::Error for PasswordError {}

   pub struct PasswordService;

   impl PasswordService {
     pub fn generate(password: &str) -> Result<String, PasswordError> {
       // Implementation with Argon2id, OsRng salt generation
     }

     pub fn verify(password: &str, hash: &str) -> Result<bool, PasswordError> {
       // Implementation with constant-time comparison
     }
   }

   #[cfg(test)]
   mod tests {
     // Comprehensive test suite
   }
   ```

### Modified Files
1. **`src/services/mod.rs`** - Add password service exports
2. **`Cargo.toml`** - Add argon2 and rand dependencies

## Testing Strategy

### Unit Test Coverage
- **Functionality**: 100% coverage of public methods
- **Error Cases**: All error paths tested
- **Edge Cases**: Empty passwords, long passwords, Unicode
- **Security**: Salt uniqueness, hash format validation

### Integration Considerations
- Service is self-contained and doesn't require database
- No integration tests needed for this phase
- Future phases will test integration with user management

## Success Criteria

- [ ] `PasswordService` successfully generates secure password hashes
- [ ] Generated hashes use different salts for the same password
- [ ] `verify` method correctly validates matching passwords
- [ ] `verify` method correctly rejects non-matching passwords
- [ ] Proper error handling for invalid inputs
- [ ] Comprehensive unit test coverage (>95%)
- [ ] All tests pass consistently
- [ ] Documentation follows project standards
- [ ] Code follows 2-space indentation standard

## Security Validation

- [ ] Verify salt uniqueness across multiple hash generations
- [ ] Confirm Argon2id algorithm usage
- [ ] Validate 16-byte salt length
- [ ] Test hash format compliance with PHC string format
- [ ] Ensure no plaintext passwords in logs or error messages

## Future Integration Points

This implementation prepares for Phase 4 (Database Configuration) where:
- Password hashes will be stored in the users table
- User registration will use `PasswordService::generate`
- User authentication will use `PasswordService::verify`
- The service interface remains unchanged for database integration