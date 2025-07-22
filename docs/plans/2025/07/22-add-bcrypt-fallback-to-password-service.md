# Add Bcrypt Fallback to PasswordService::verify

**Date**: 2025-07-22@09:15  
**Status**: Planning

## Overview

Add Bcrypt fallback support to the `PasswordService::verify` method to enable migration from existing Bcrypt password hashes while maintaining Argon2 for new password generation. The implementation should automatically detect the hash algorithm based on the hash string format.

## Background

The current `PasswordService` only supports Argon2id for both password generation and verification. To support migration scenarios where existing systems have Bcrypt password hashes (e.g., `$2b$10$bCTECoMkzgc.2Hx1fLurIe0jETMO318OWpdmBwDnt03uE2GepN8kS...`), we need to add fallback verification support while keeping Argon2 as the primary algorithm for new passwords.

## Requirements

1. **Password Generation**: Continue using Argon2id exclusively for new password hashes
2. **Password Verification**: Support both Argon2id and Bcrypt hash verification
3. **Algorithm Detection**: Automatically determine the algorithm based on hash string prefix
4. **Backward Compatibility**: Maintain existing API without breaking changes
5. **Error Handling**: Provide clear error messages for unsupported hash formats
6. **Performance**: Ensure verification performance remains acceptable for both algorithms

## Hash Format Detection

- **Argon2id**: `$argon2id$v=19$m=4096,t=3,p=1$...` (current format)
- **Bcrypt**: `$2b$10$...` (target fallback format)

## Implementation Plan

### Phase 1: Add Bcrypt Dependency

**Files to modify:**
- `Cargo.toml`

**Changes:**
- Add `bcrypt` crate dependency (version `~0.15`)

### Phase 2: Enhance PasswordService::verify Method

**Files to modify:**
- `src/services/password_service.rs`

**Changes:**

1. **Add hash algorithm detection function:**
```rust
impl PasswordService {
  /// Detects the password hashing algorithm based on hash format
  fn detect_algorithm(hash: &str) -> Result<HashAlgorithm, PasswordError> {
    if hash.starts_with("$argon2id$") || hash.starts_with("$argon2i$") || hash.starts_with("$argon2d$") {
      Ok(HashAlgorithm::Argon2)
    } else if hash.starts_with("$2b$") || hash.starts_with("$2a$") || hash.starts_with("$2y$") {
      Ok(HashAlgorithm::Bcrypt)
    } else {
      Err(PasswordError::InvalidHash(format!("Unsupported hash format: {}", &hash[..std::cmp::min(20, hash.len())])))
    }
  }
}

enum HashAlgorithm {
  Argon2,
  Bcrypt,
}
```

2. **Update verify method to support both algorithms:**
```rust
pub fn verify(password: &str, hash: &str) -> Result<bool, PasswordError> {
  match Self::detect_algorithm(hash)? {
    HashAlgorithm::Argon2 => Self::verify_argon2(password, hash),
    HashAlgorithm::Bcrypt => Self::verify_bcrypt(password, hash),
  }
}

fn verify_argon2(password: &str, hash: &str) -> Result<bool, PasswordError> {
  // Current Argon2 verification logic (extracted from existing verify method)
}

fn verify_bcrypt(password: &str, hash: &str) -> Result<bool, PasswordError> {
  // New Bcrypt verification logic using bcrypt crate
}
```

3. **Add import statements:**
```rust
use bcrypt;
```

### Phase 3: Comprehensive Testing

**Files to modify:**
- `src/services/password_service.rs` (test module)

**New test cases:**

1. **Algorithm detection tests:**
```rust
#[test]
fn test_detect_argon2_algorithm() {
  // Test various Argon2 hash formats
}

#[test]
fn test_detect_bcrypt_algorithm() {
  // Test various Bcrypt hash formats
}

#[test]
fn test_detect_unsupported_algorithm() {
  // Test unsupported hash formats
}
```

2. **Bcrypt verification tests:**
```rust
#[test]
fn test_verify_bcrypt_correct_password() {
  // Test with known Bcrypt hash and correct password
}

#[test]
fn test_verify_bcrypt_incorrect_password() {
  // Test with known Bcrypt hash and incorrect password
}

#[test]
fn test_verify_bcrypt_various_cost_factors() {
  // Test Bcrypt hashes with different cost factors (10, 12, etc.)
}
```

3. **Integration tests:**
```rust
#[test]
fn test_verify_mixed_hash_types() {
  // Test that both Argon2 and Bcrypt hashes work in the same test
}

#[test]
fn test_backward_compatibility() {
  // Ensure existing Argon2 verification still works exactly as before
}
```

4. **Edge case tests:**
```rust
#[test]
fn test_verify_malformed_bcrypt_hash() {
  // Test with malformed Bcrypt hashes
}

#[test]
fn test_verify_empty_bcrypt_hash() {
  // Test with empty hash string
}
```

### Phase 4: Documentation Updates

**Files to modify:**
- `src/services/password_service.rs`

**Changes:**
- Update doc comments for `verify` method to mention Bcrypt fallback support
- Add examples showing both Argon2 and Bcrypt hash verification
- Update module-level documentation to explain the dual-algorithm support

**Example documentation update:**
```rust
/// Verifies a password against a previously generated hash
///
/// This method supports both Argon2id (primary) and Bcrypt (fallback) hash formats.
/// The algorithm is automatically detected based on the hash string prefix:
/// - Argon2: `$argon2id$`, `$argon2i$`, `$argon2d$`
/// - Bcrypt: `$2b$`, `$2a$`, `$2y$`
///
/// # Arguments
///
/// * `password` - The plaintext password to verify
/// * `hash` - The encoded hash string to verify against (Argon2 or Bcrypt format)
///
/// # Returns
///
/// * `Ok(true)` - If the password matches the hash
/// * `Ok(false)` - If the password does not match the hash
/// * `Err(PasswordError)` - If verification fails due to invalid/unsupported hash format
///
/// # Examples
///
/// ```
/// use dp_auth_service::services::PasswordService;
///
/// // Argon2 hash (primary algorithm)
/// let argon2_hash = PasswordService::generate("my_password").unwrap();
/// let is_valid = PasswordService::verify("my_password", &argon2_hash).unwrap();
/// assert!(is_valid);
///
/// // Bcrypt hash (fallback for migration)
/// let bcrypt_hash = "$2b$10$bCTECoMkzgc.2Hx1fLurIe0jETMO318OWpdmBwDnt03uE2GepN8kS";
/// let is_valid = PasswordService::verify("correct_password", bcrypt_hash).unwrap();
/// // Result depends on whether "correct_password" matches the hash
/// ```
pub fn verify(password: &str, hash: &str) -> Result<bool, PasswordError> {
```

## Files Summary

### Files to be Created
- None (all changes are modifications to existing files)

### Files to be Modified
1. **`Cargo.toml`**
   - Add `bcrypt` dependency

2. **`src/services/password_service.rs`**
   - Add `HashAlgorithm` enum
   - Add `detect_algorithm` method
   - Refactor `verify` method to support both algorithms
   - Add `verify_argon2` and `verify_bcrypt` helper methods
   - Add comprehensive test cases for Bcrypt support
   - Update documentation

## Migration Strategy

This implementation provides a seamless migration path:

1. **Existing systems**: Can immediately start using the updated service to verify both Argon2 and Bcrypt hashes
2. **New passwords**: Will continue to be generated using Argon2id
3. **Legacy passwords**: Will be verified using Bcrypt when detected
4. **Future migration**: Systems can gradually migrate users to Argon2 by prompting password changes

## Security Considerations

1. **Algorithm preference**: Argon2id remains the primary algorithm for new hashes
2. **Bcrypt support**: Limited to verification only, not generation
3. **Hash detection**: Based on well-established format prefixes
4. **Error handling**: Clear distinction between algorithm detection failures and verification failures
5. **Performance**: Bcrypt verification performance is acceptable for authentication use cases

## Testing Strategy

1. **Unit tests**: Comprehensive coverage of both algorithms and edge cases
2. **Integration tests**: Verify the service works correctly in the broader application context
3. **Performance tests**: Ensure verification times remain acceptable for both algorithms
4. **Security tests**: Verify timing attack resistance is maintained

## Risks and Mitigation

1. **Risk**: Bcrypt dependency adds attack surface
   - **Mitigation**: Use well-maintained `bcrypt` crate with good security track record

2. **Risk**: Performance degradation
   - **Mitigation**: Algorithm detection is fast (string prefix check), and Bcrypt verification is only used when needed

3. **Risk**: Breaking changes to existing API
   - **Mitigation**: No API changes, only internal implementation changes

## Success Criteria

1. All existing tests continue to pass
2. New Bcrypt verification tests pass
3. Performance remains within acceptable bounds
4. Documentation clearly explains the dual-algorithm support
5. Integration tests demonstrate seamless operation with both hash types