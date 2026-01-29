use crate::types::PasswordError;
use argon2::{
  password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
  Argon2, Params,
};

pub struct GeneratePasswordHashService;

impl GeneratePasswordHashService {
  pub fn run(password: &str) -> Result<String, PasswordError> {
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
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate_creates_valid_hash() {
    let password = "test_password_123";
    let hash = GeneratePasswordHashService::run(password).unwrap();

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
    let hash1 = GeneratePasswordHashService::run(password).unwrap();
    let hash2 = GeneratePasswordHashService::run(password).unwrap();

    // Hashes should be different due to different salts
    assert_ne!(hash1, hash2);

    // Both should be valid Argon2id hashes
    assert!(hash1.starts_with("$argon2id$"));
    assert!(hash2.starts_with("$argon2id$"));
  }

  #[test]
  fn test_hash_format_contains_expected_components() {
    let password = "test_password";
    let hash = GeneratePasswordHashService::run(password).unwrap();

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
}
