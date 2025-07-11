use argon2::{
  password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
  Argon2, Params,
};
use std::fmt;

/// Custom error type for password operations
#[derive(Debug)]
pub enum PasswordError {
  /// Error occurred during password hashing
  HashingError(String),
  /// Error occurred during password verification
  VerificationError(String),
  /// Invalid hash format provided
  InvalidHash(String),
}

impl fmt::Display for PasswordError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      PasswordError::HashingError(msg) => write!(f, "Password hashing error: {msg}"),
      PasswordError::VerificationError(msg) => write!(f, "Password verification error: {msg}"),
      PasswordError::InvalidHash(msg) => write!(f, "Invalid hash format: {msg}"),
    }
  }
}

impl std::error::Error for PasswordError {}

/// Service for secure password hashing and verification using Argon2
pub struct PasswordService;

impl PasswordService {
  /// Generates a secure password hash using Argon2id with a random salt
  ///
  /// This method creates a new 16-byte salt using a cryptographically secure
  /// random number generator and hashes the provided password using Argon2id.
  /// The resulting hash uses 4MB memory cost and 3 time iterations for a good
  /// balance between security and performance.
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
  /// assert!(hash.starts_with("$argon2id$"));
  /// ```
  pub fn generate(password: &str) -> Result<String, PasswordError> {
    // Configure Argon2 with our chosen parameters
    let params = Params::new(
      4096,     // memory cost: 4MB
      3,        // time cost: 3 iterations
      1,        // parallelism: 1 thread
      Some(32), // hash length: 32 bytes
    )
    .map_err(|e| PasswordError::HashingError(format!("Failed to create Argon2 params: {e}")))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    // Generate a random 16-byte salt
    let salt = SaltString::generate(&mut OsRng);

    // Hash the password
    let password_hash = argon2
      .hash_password(password.as_bytes(), &salt)
      .map_err(|e| PasswordError::HashingError(format!("Failed to hash password: {e}")))?;

    Ok(password_hash.to_string())
  }

  /// Verifies a password against a previously generated hash
  ///
  /// This method extracts the salt and parameters from the encoded hash
  /// and verifies the provided password against it using constant-time comparison.
  /// The hash format is automatically parsed to extract the original parameters,
  /// ensuring compatibility with hashes generated using different parameter sets.
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
  ///
  /// let is_invalid = PasswordService::verify("wrong_password", &hash).unwrap();
  /// assert!(!is_invalid);
  /// ```
  pub fn verify(password: &str, hash: &str) -> Result<bool, PasswordError> {
    // Parse the hash string to extract parameters and salt
    let parsed_hash = PasswordHash::new(hash)
      .map_err(|e| PasswordError::InvalidHash(format!("Failed to parse hash: {e}")))?;

    // Create Argon2 instance (parameters will be extracted from the hash)
    let argon2 = Argon2::default();

    // Verify the password against the hash
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
      Ok(()) => Ok(true),
      Err(argon2::password_hash::Error::Password) => Ok(false),
      Err(e) => Err(PasswordError::VerificationError(format!(
        "Failed to verify password: {e}"
      ))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate_creates_valid_hash() {
    let password = "test_password_123";
    let hash = PasswordService::generate(password).unwrap();

    // Hash should not be empty
    assert!(!hash.is_empty());

    // Hash should start with Argon2id identifier
    assert!(hash.starts_with("$argon2id$"));

    // Hash should contain our parameters
    assert!(hash.contains("m=4096"));
    assert!(hash.contains("t=3"));
    assert!(hash.contains("p=1"));
  }

  #[test]
  fn test_generate_different_salts() {
    let password = "same_password";
    let hash1 = PasswordService::generate(password).unwrap();
    let hash2 = PasswordService::generate(password).unwrap();

    // Hashes should be different due to different salts
    assert_ne!(hash1, hash2);

    // Both should be valid Argon2id hashes
    assert!(hash1.starts_with("$argon2id$"));
    assert!(hash2.starts_with("$argon2id$"));
  }

  #[test]
  fn test_verify_correct_password() {
    let password = "correct_password";
    let hash = PasswordService::generate(password).unwrap();

    let is_valid = PasswordService::verify(password, &hash).unwrap();
    assert!(is_valid);
  }

  #[test]
  fn test_verify_incorrect_password() {
    let password = "correct_password";
    let wrong_password = "wrong_password";
    let hash = PasswordService::generate(password).unwrap();

    let is_valid = PasswordService::verify(wrong_password, &hash).unwrap();
    assert!(!is_valid);
  }

  #[test]
  fn test_verify_invalid_hash_format() {
    let password = "any_password";
    let invalid_hash = "not_a_valid_hash";

    let result = PasswordService::verify(password, invalid_hash);
    assert!(result.is_err());

    match result.unwrap_err() {
      PasswordError::InvalidHash(_) => {} // Expected
      _ => panic!("Expected InvalidHash error"),
    }
  }

  #[test]
  fn test_empty_password_handling() {
    let empty_password = "";
    let hash = PasswordService::generate(empty_password).unwrap();

    // Should be able to generate hash for empty password
    assert!(!hash.is_empty());

    // Should be able to verify empty password
    let is_valid = PasswordService::verify(empty_password, &hash).unwrap();
    assert!(is_valid);

    // Wrong password should still fail
    let is_invalid = PasswordService::verify("not_empty", &hash).unwrap();
    assert!(!is_invalid);
  }

  #[test]
  fn test_very_long_password_handling() {
    let long_password = "a".repeat(1000); // 1000 character password
    let hash = PasswordService::generate(&long_password).unwrap();

    // Should handle long passwords
    assert!(!hash.is_empty());

    let is_valid = PasswordService::verify(&long_password, &hash).unwrap();
    assert!(is_valid);
  }

  #[test]
  fn test_unicode_password_handling() {
    let unicode_password = "пароль🔒密码";
    let hash = PasswordService::generate(unicode_password).unwrap();

    // Should handle Unicode characters
    assert!(!hash.is_empty());

    let is_valid = PasswordService::verify(unicode_password, &hash).unwrap();
    assert!(is_valid);
  }

  #[test]
  fn test_special_characters_in_password() {
    let special_password = "p@$$w0rd!#$%^&*()";
    let hash = PasswordService::generate(special_password).unwrap();

    let is_valid = PasswordService::verify(special_password, &hash).unwrap();
    assert!(is_valid);
  }

  #[test]
  fn test_verify_with_empty_hash_returns_error() {
    let password = "any_password";
    let empty_hash = "";

    let result = PasswordService::verify(password, empty_hash);
    assert!(result.is_err());
  }

  #[test]
  fn test_hash_format_contains_expected_components() {
    let password = "test_password";
    let hash = PasswordService::generate(password).unwrap();

    // Split hash into components
    let parts: Vec<&str> = hash.split('$').collect();

    // Should have format: $argon2id$v=19$m=4096,t=3,p=1$salt$hash
    assert!(parts.len() >= 5);
    assert_eq!(parts[1], "argon2id");
    assert_eq!(parts[2], "v=19");
    assert!(parts[3].contains("m=4096"));
    assert!(parts[3].contains("t=3"));
    assert!(parts[3].contains("p=1"));
  }

  #[test]
  fn test_verify_timing_consistency() {
    use std::time::Instant;

    let password = "test_password";
    let hash = PasswordService::generate(password).unwrap();

    // Measure time for correct password
    let start = Instant::now();
    let _result1 = PasswordService::verify(password, &hash).unwrap();
    let time1 = start.elapsed();

    // Measure time for incorrect password
    let start = Instant::now();
    let _result2 = PasswordService::verify("wrong_password", &hash).unwrap();
    let time2 = start.elapsed();

    // Times should be reasonably similar (within an order of magnitude)
    // This is a basic check - proper timing attack resistance would need more sophisticated testing
    let ratio = if time1 > time2 {
      time1.as_nanos() as f64 / time2.as_nanos() as f64
    } else {
      time2.as_nanos() as f64 / time1.as_nanos() as f64
    };

    // Allow up to 10x difference (very generous for basic timing consistency)
    assert!(ratio < 10.0, "Timing difference too large: {ratio}x");
  }

  #[test]
  fn test_performance_benchmarks() {
    use std::time::Instant;

    let password = "performance_test_password";

    // Test generate performance
    let start = Instant::now();
    let hash = PasswordService::generate(password).unwrap();
    let generate_time = start.elapsed();

    // Test verify performance
    let start = Instant::now();
    let result = PasswordService::verify(password, &hash).unwrap();
    let verify_time = start.elapsed();

    assert!(result);

    // Print timing for manual verification (only in debug builds)
    #[cfg(debug_assertions)]
    {
      println!("Generate time: {generate_time:?}");
      println!("Verify time: {verify_time:?}");
      println!("Hash format: {hash}");
    }

    // Verify timing is reasonable (10ms to 1000ms range)
    // Lower bound ensures we're actually doing work
    // Upper bound ensures it's not too slow for production
    assert!(
      generate_time.as_millis() >= 10,
      "Generate too fast, might not be secure"
    );
    assert!(
      generate_time.as_millis() <= 1000,
      "Generate too slow for production"
    );
    assert!(
      verify_time.as_millis() >= 10,
      "Verify too fast, might not be secure"
    );
    assert!(
      verify_time.as_millis() <= 1000,
      "Verify too slow for production"
    );
  }
}
