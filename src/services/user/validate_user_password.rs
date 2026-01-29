use crate::types::UserError;

pub struct ValidateUserPasswordService;

impl ValidateUserPasswordService {
  pub fn run(password: &str) -> Result<(), UserError> {
    if password.is_empty() {
      return Err(UserError::ValidationError(
        "Password cannot be empty".to_string(),
      ));
    }

    if password.len() < 6 {
      return Err(UserError::ValidationError(
        "Password must be at least 6 characters long".to_string(),
      ));
    }

    if password.len() > 128 {
      return Err(UserError::ValidationError(
        "Password cannot be longer than 128 characters".to_string(),
      ));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_validate_password_empty() {
    let result = ValidateUserPasswordService::run("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[tokio::test]
  async fn test_validate_password_too_short() {
    let result = ValidateUserPasswordService::run("12345");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("at least 6 characters"));
  }

  #[tokio::test]
  async fn test_validate_password_too_long() {
    let result = ValidateUserPasswordService::run(&"a".repeat(129));
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("longer than 128 characters"));
  }

  #[tokio::test]
  async fn test_validate_password_valid() {
    assert!(ValidateUserPasswordService::run("123456").is_ok());
    assert!(ValidateUserPasswordService::run("password123").is_ok());
    assert!(ValidateUserPasswordService::run(&"a".repeat(128)).is_ok());
  }
}
