use base64::{engine::general_purpose, Engine as _};

/// Error type for secret reading operations
#[derive(Debug, Clone, PartialEq)]
pub enum SecretError {
  /// Environment variable not found
  NotFound(String),
  /// Base64 decoding failed
  InvalidBase64(String),
  /// Secret too short for cryptographic use
  TooShort { actual: usize, minimum: usize },
  /// Secret cache already initialized
  AlreadyInitialized,
}

impl std::fmt::Display for SecretError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SecretError::NotFound(var_name) => {
        write!(f, "Environment variable '{var_name}' not found")
      }
      SecretError::InvalidBase64(var_name) => {
        write!(
          f,
          "Environment variable '{var_name}' contains invalid base64"
        )
      }
      SecretError::TooShort { actual, minimum } => {
        write!(
          f,
          "Secret too short: {actual} bytes, need at least {minimum}"
        )
      }
      SecretError::AlreadyInitialized => {
        write!(f, "Secret cache already initialized")
      }
    }
  }
}

impl std::error::Error for SecretError {}

/// Read and validate a base64-encoded secret from an environment variable
///
/// This function reads a secret from the specified environment variable,
/// decodes it from base64, and validates that it meets the minimum length
/// requirement for cryptographic use.
///
/// # Arguments
///
/// * `env_var_name` - Name of the environment variable to read
/// * `min_bytes` - Minimum number of bytes required (typically 32 for AES-256)
///
/// # Returns
///
/// Returns exactly `min_bytes` bytes from the decoded secret, or an error if:
/// - The environment variable is not set
/// - The value is not valid base64
/// - The decoded secret is shorter than `min_bytes`
///
/// # Examples
///
/// ```rust
/// use dp_auth_service::utils::get_secret_from_env::get_secret_from_env;
/// use std::env;
///
/// // Set up a test environment variable
/// env::set_var("MY_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");
///
/// // Read a 32-byte AES-256 key
/// let secret = get_secret_from_env("MY_SECRET_KEY", 32)?;
/// assert_eq!(secret.len(), 32);
///
/// // Clean up
/// env::remove_var("MY_SECRET_KEY");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn get_secret_from_env(env_var_name: &str, min_bytes: usize) -> Result<Vec<u8>, SecretError> {
  // Read environment variable
  let key_str =
    std::env::var(env_var_name).map_err(|_| SecretError::NotFound(env_var_name.to_string()))?;

  // Decode base64
  let key_bytes = general_purpose::STANDARD
    .decode(&key_str)
    .map_err(|_| SecretError::InvalidBase64(env_var_name.to_string()))?;

  // Validate minimum length
  if key_bytes.len() < min_bytes {
    return Err(SecretError::TooShort {
      actual: key_bytes.len(),
      minimum: min_bytes,
    });
  }

  // Return exactly the requested number of bytes
  Ok(key_bytes[..min_bytes].to_vec())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::env;

  #[test]
  fn test_get_secret_from_env_success() {
    let test_var = "TEST_SECRET_SUCCESS";
    let test_secret = "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg="; // 32 bytes when decoded
    env::set_var(test_var, test_secret);

    let result = get_secret_from_env(test_var, 32);
    assert!(result.is_ok());
    let secret = result.unwrap();
    assert_eq!(secret.len(), 32);

    env::remove_var(test_var);
  }

  #[test]
  fn test_get_secret_from_env_missing_var() {
    let test_var = "TEST_SECRET_MISSING";
    env::remove_var(test_var);

    let result = get_secret_from_env(test_var, 32);
    assert!(matches!(result, Err(SecretError::NotFound(_))));
    assert_eq!(
      result.unwrap_err(),
      SecretError::NotFound(test_var.to_string())
    );
  }

  #[test]
  fn test_get_secret_from_env_invalid_base64() {
    let test_var = "TEST_SECRET_INVALID_BASE64";
    env::set_var(test_var, "invalid-base64!");

    let result = get_secret_from_env(test_var, 32);
    assert!(matches!(result, Err(SecretError::InvalidBase64(_))));
    assert_eq!(
      result.unwrap_err(),
      SecretError::InvalidBase64(test_var.to_string())
    );

    env::remove_var(test_var);
  }

  #[test]
  fn test_get_secret_from_env_too_short() {
    let test_var = "TEST_SECRET_TOO_SHORT";
    let short_secret = "c2hvcnQ="; // "short" in base64 (5 bytes)
    env::set_var(test_var, short_secret);

    let result = get_secret_from_env(test_var, 32);
    assert!(matches!(result, Err(SecretError::TooShort { .. })));
    if let Err(SecretError::TooShort { actual, minimum }) = result {
      assert_eq!(actual, 5);
      assert_eq!(minimum, 32);
    }

    env::remove_var(test_var);
  }

  #[test]
  fn test_get_secret_from_env_exact_minimum_length() {
    let test_var = "TEST_SECRET_EXACT_LENGTH";
    // Create a secret that's exactly 16 bytes when decoded
    let exact_secret = "MTIzNDU2Nzg5MDEyMzQ1Ng=="; // "1234567890123456" in base64 (16 bytes)
    env::set_var(test_var, exact_secret);

    let result = get_secret_from_env(test_var, 16);
    assert!(result.is_ok());
    let secret = result.unwrap();
    assert_eq!(secret.len(), 16);

    env::remove_var(test_var);
  }

  #[test]
  fn test_get_secret_from_env_longer_than_minimum() {
    let test_var = "TEST_SECRET_LONGER";
    // Create a valid base64 string that decodes to more than 32 bytes (80 bytes)
    let long_secret = "VGhpcyBpcyBhIHRlc3Qgc2VjcmV0IHRoYXQgaXMgZGVmaW5pdGVseSBsb25nZXIgdGhhbiAzMiBieXRlcyBmb3IgdGVzdGluZyBwdXJwb3Nlcw==";
    env::set_var(test_var, long_secret);

    let result = get_secret_from_env(test_var, 32);
    assert!(result.is_ok());
    let secret = result.unwrap();
    assert_eq!(secret.len(), 32); // Should return exactly 32 bytes

    env::remove_var(test_var);
  }
}
