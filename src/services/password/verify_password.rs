use crate::types::PasswordError;
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use bcrypt;

#[derive(Debug, PartialEq)]
enum HashAlgorithm {
  Argon2,
  Bcrypt,
}

pub struct VerifyPasswordService;

impl VerifyPasswordService {
  pub fn run(password: &str, hash: &str) -> Result<bool, PasswordError> {
    match Self::detect_algorithm(hash)? {
      HashAlgorithm::Argon2 => Self::verify_argon2(password, hash),
      HashAlgorithm::Bcrypt => Self::verify_bcrypt(password, hash),
    }
  }

  fn detect_algorithm(hash: &str) -> Result<HashAlgorithm, PasswordError> {
    if hash.starts_with("$argon2id$")
      || hash.starts_with("$argon2i$")
      || hash.starts_with("$argon2d$")
    {
      Ok(HashAlgorithm::Argon2)
    } else if hash.starts_with("$2b$") || hash.starts_with("$2a$") || hash.starts_with("$2y$") {
      Ok(HashAlgorithm::Bcrypt)
    } else {
      let preview = if hash.len() > 20 {
        format!("{}...", &hash[..20])
      } else {
        hash.to_string()
      };
      Err(PasswordError::InvalidHash(format!(
        "Unsupported hash format: {preview}"
      )))
    }
  }

  fn verify_argon2(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash = PasswordHash::new(hash)
      .map_err(|e| PasswordError::InvalidHash(format!("Failed to parse Argon2 hash: {e}")))?;

    let argon2 = Argon2::default();

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
      Ok(()) => Ok(true),
      Err(argon2::password_hash::Error::Password) => Ok(false),
      Err(e) => Err(PasswordError::VerificationError(format!(
        "Failed to verify Argon2 password: {e}"
      ))),
    }
  }

  fn verify_bcrypt(password: &str, hash: &str) -> Result<bool, PasswordError> {
    match bcrypt::verify(password, hash) {
      Ok(is_valid) => Ok(is_valid),
      Err(e) => Err(PasswordError::VerificationError(format!(
        "Failed to verify Bcrypt password: {e}"
      ))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::GeneratePasswordHashService;
  use std::time::Instant;

  #[test]
  fn test_detect_argon2_algorithm() {
    assert_eq!(
      VerifyPasswordService::detect_algorithm("$argon2id$v=19$m=4096,t=3,p=1$salt$hash").unwrap(),
      HashAlgorithm::Argon2
    );
    assert_eq!(
      VerifyPasswordService::detect_algorithm("$argon2i$v=19$m=4096,t=3,p=1$salt$hash").unwrap(),
      HashAlgorithm::Argon2
    );
    assert_eq!(
      VerifyPasswordService::detect_algorithm("$argon2d$v=19$m=4096,t=3,p=1$salt$hash").unwrap(),
      HashAlgorithm::Argon2
    );
  }

  #[test]
  fn test_detect_bcrypt_algorithm() {
    assert_eq!(
      VerifyPasswordService::detect_algorithm(
        "$2b$10$bCTECoMkzgc.2Hx1fLurIe0jETMO318OWpdmBwDnt03uE2GepN8kS"
      )
      .unwrap(),
      HashAlgorithm::Bcrypt
    );
    assert_eq!(
      VerifyPasswordService::detect_algorithm("$2a$12$salt.and.hash.here").unwrap(),
      HashAlgorithm::Bcrypt
    );
    assert_eq!(
      VerifyPasswordService::detect_algorithm("$2y$10$another.bcrypt.hash").unwrap(),
      HashAlgorithm::Bcrypt
    );
  }

  #[test]
  fn test_detect_unsupported_algorithm() {
    let result = VerifyPasswordService::detect_algorithm("$md5$unsupported");
    assert!(result.is_err());
    match result.unwrap_err() {
      PasswordError::InvalidHash(msg) => {
        assert!(msg.contains("Unsupported hash format"));
        assert!(msg.contains("$md5$unsupported"));
      }
      _ => panic!("Expected InvalidHash error"),
    }

    let long_hash = "$unsupported$".to_string() + &"x".repeat(100);
    let result = VerifyPasswordService::detect_algorithm(&long_hash);
    assert!(result.is_err());
    match result.unwrap_err() {
      PasswordError::InvalidHash(msg) => {
        assert!(msg.contains("Unsupported hash format"));
        assert!(msg.contains("..."));
        assert!(msg.len() < long_hash.len());
      }
      _ => panic!("Expected InvalidHash error"),
    }
  }

  #[test]
  fn test_verify_invalid_hash_format() {
    let password = "any_password";
    let invalid_hash = "not_a_valid_hash";

    let result = VerifyPasswordService::run(password, invalid_hash);
    assert!(result.is_err());

    match result.unwrap_err() {
      PasswordError::InvalidHash(_) => {}
      _ => panic!("Expected InvalidHash error"),
    }
  }

  #[test]
  fn test_verify_correct_password() {
    let password = "correct_password";
    let hash = GeneratePasswordHashService::run(password).unwrap();

    let is_valid = VerifyPasswordService::run(password, &hash).unwrap();
    assert!(is_valid);
  }

  #[test]
  fn test_verify_incorrect_password() {
    let password = "correct_password";
    let wrong_password = "wrong_password";
    let hash = GeneratePasswordHashService::run(password).unwrap();

    let is_valid = VerifyPasswordService::run(wrong_password, &hash).unwrap();
    assert!(!is_valid);
  }

  #[test]
  fn test_empty_password_handling() {
    let empty_password = "";
    let hash = GeneratePasswordHashService::run(empty_password).unwrap();

    assert!(!hash.is_empty());

    let is_valid = VerifyPasswordService::run(empty_password, &hash).unwrap();
    assert!(is_valid);

    let is_invalid = VerifyPasswordService::run("not_empty", &hash).unwrap();
    assert!(!is_invalid);
  }

  #[test]
  fn test_very_long_password_handling() {
    let long_password = "a".repeat(1000);
    let hash = GeneratePasswordHashService::run(&long_password).unwrap();

    assert!(!hash.is_empty());

    let is_valid = VerifyPasswordService::run(&long_password, &hash).unwrap();
    assert!(is_valid);
  }

  #[test]
  fn test_unicode_password_handling() {
    let unicode_password = "пароль🔒密码";
    let hash = GeneratePasswordHashService::run(unicode_password).unwrap();

    assert!(!hash.is_empty());

    let is_valid = VerifyPasswordService::run(unicode_password, &hash).unwrap();
    assert!(is_valid);
  }

  #[test]
  fn test_special_characters_in_password() {
    let special_password = "p@$$w0rd!#$%^&*()";
    let hash = GeneratePasswordHashService::run(special_password).unwrap();

    let is_valid = VerifyPasswordService::run(special_password, &hash).unwrap();
    assert!(is_valid);
  }

  #[test]
  fn test_verify_timing_consistency() {
    let password = "test_password";
    let hash = GeneratePasswordHashService::run(password).unwrap();

    let start = Instant::now();
    let _result1 = VerifyPasswordService::run(password, &hash).unwrap();
    let time1 = start.elapsed();

    let start = Instant::now();
    let _result2 = VerifyPasswordService::run("wrong_password", &hash).unwrap();
    let time2 = start.elapsed();

    let ratio = if time1 > time2 {
      time1.as_nanos() as f64 / time2.as_nanos() as f64
    } else {
      time2.as_nanos() as f64 / time1.as_nanos() as f64
    };

    assert!(ratio < 10.0, "Timing difference too large: {ratio}x");
  }

  #[test]
  fn test_verify_with_empty_hash_returns_error() {
    let password = "any_password";
    let empty_hash = "";

    let result = VerifyPasswordService::run(password, empty_hash);
    assert!(result.is_err());
  }

  #[test]
  fn test_verify_bcrypt_correct_password() {
    let bcrypt_hash = "$2b$10$Ak/lFZ/V.YZ74FXS9y3u1.2ZNK4Ae4xhUDpmVLy.tr2JX8.KE/KuO";
    let password = "test_password";

    let result = VerifyPasswordService::run(password, bcrypt_hash).unwrap();
    assert!(result);
  }

  #[test]
  fn test_verify_bcrypt_incorrect_password() {
    let bcrypt_hash = "$2b$10$Ak/lFZ/V.YZ74FXS9y3u1.2ZNK4Ae4xhUDpmVLy.tr2JX8.KE/KuO";
    let wrong_password = "wrong_password";

    let result = VerifyPasswordService::run(wrong_password, bcrypt_hash).unwrap();
    assert!(!result);
  }

  #[test]
  fn test_verify_malformed_bcrypt_hash() {
    let malformed_hashes = vec![
      "$2b$10$",
      "$2b$10$invalid",
      "$2b$99$N9qo8uLOickgx2ZMRZoMye.IjdBqjdqHZc8KQqZF6f6.6uYqKQYyO",
    ];

    for malformed_hash in malformed_hashes {
      let result = VerifyPasswordService::run("any_password", malformed_hash);
      assert!(
        result.is_err(),
        "Should fail for malformed hash: {malformed_hash}"
      );
      match result.unwrap_err() {
        PasswordError::VerificationError(_) => {}
        _ => panic!("Expected VerificationError for malformed hash: {malformed_hash}"),
      }
    }
  }

  #[test]
  fn test_performance_benchmarks() {
    let password = "performance_test_password";

    let start = Instant::now();
    let hash = GeneratePasswordHashService::run(password).unwrap();
    let generate_time = start.elapsed();

    let start = Instant::now();
    let result = VerifyPasswordService::run(password, &hash).unwrap();
    let verify_time = start.elapsed();

    assert!(result);

    #[cfg(debug_assertions)]
    {
      println!("Generate time: {generate_time:?}");
      println!("Verify time: {verify_time:?}");
      println!("Hash format: {hash}");
    }

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

  #[test]
  fn test_verify_bcrypt_various_cost_factors() {
    let hash_cost4 = "$2b$04$Q.8Z8uLOickgx2ZMRZoMye.IjdBqjdqHZc8KQqZF6f6.6uYqKQYyO";
    assert_eq!(
      VerifyPasswordService::detect_algorithm(hash_cost4).unwrap(),
      HashAlgorithm::Bcrypt
    );

    let hash_cost12 = "$2b$12$Q.8Z8uLOickgx2ZMRZoMye.IjdBqjdqHZc8KQqZF6f6.6uYqKQYyO";
    assert_eq!(
      VerifyPasswordService::detect_algorithm(hash_cost12).unwrap(),
      HashAlgorithm::Bcrypt
    );
  }

  #[test]
  fn test_verify_mixed_hash_types() {
    let password = "mixed_test_password";

    let argon2_hash = GeneratePasswordHashService::run(password).unwrap();
    let argon2_result = VerifyPasswordService::run(password, &argon2_hash).unwrap();
    assert!(argon2_result);

    let bcrypt_hash = "$2b$10$Ak/lFZ/V.YZ74FXS9y3u1.2ZNK4Ae4xhUDpmVLy.tr2JX8.KE/KuO";
    let bcrypt_result = VerifyPasswordService::run("test_password", bcrypt_hash).unwrap();
    assert!(bcrypt_result);

    let argon2_wrong = VerifyPasswordService::run("wrong", &argon2_hash).unwrap();
    assert!(!argon2_wrong);

    let bcrypt_wrong = VerifyPasswordService::run("wrong", bcrypt_hash).unwrap();
    assert!(!bcrypt_wrong);
  }

  #[test]
  fn test_backward_compatibility() {
    let password = "backward_compatibility_test";
    let hash = GeneratePasswordHashService::run(password).unwrap();

    let is_valid = VerifyPasswordService::run(password, &hash).unwrap();
    assert!(is_valid);

    let is_invalid = VerifyPasswordService::run("wrong_password", &hash).unwrap();
    assert!(!is_invalid);

    assert!(hash.starts_with("$argon2id$"));
  }

  #[test]
  fn test_verify_empty_bcrypt_hash() {
    let result = VerifyPasswordService::run("any_password", "");
    assert!(result.is_err());
    match result.unwrap_err() {
      PasswordError::InvalidHash(_) => {}
      _ => panic!("Expected InvalidHash error for empty hash"),
    }
  }
}
