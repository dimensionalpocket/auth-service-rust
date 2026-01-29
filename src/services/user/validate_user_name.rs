use crate::types::UserError;

pub struct ValidateUserNameService;

impl ValidateUserNameService {
  pub fn run(username: &str) -> Result<(), UserError> {
    if username.trim().is_empty() {
      return Err(UserError::ValidationError(
        "Username cannot be empty".to_string(),
      ));
    }

    if username.len() < 3 {
      return Err(UserError::ValidationError(
        "Username must be at least 3 characters long".to_string(),
      ));
    }

    if username.len() > 20 {
      return Err(UserError::ValidationError(
        "Username cannot be longer than 20 characters".to_string(),
      ));
    }

    // Check for valid characters (alphanumeric, underscore, hyphen)
    if !username
      .chars()
      .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
      return Err(UserError::ValidationError(
        "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
      ));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_validate_username_empty() {
    let result = ValidateUserNameService::run("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[tokio::test]
  async fn test_validate_username_whitespace_only() {
    let result = ValidateUserNameService::run("   ");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[tokio::test]
  async fn test_validate_username_too_short() {
    let result = ValidateUserNameService::run("ab");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("at least 3 characters"));
  }

  #[tokio::test]
  async fn test_validate_username_too_long() {
    let result = ValidateUserNameService::run("a".repeat(21).as_str());
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("longer than 20 characters"));
  }

  #[tokio::test]
  async fn test_validate_username_invalid_characters() {
    let result = ValidateUserNameService::run("test@user");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("can only contain"));
  }

  #[tokio::test]
  async fn test_validate_username_valid_characters() {
    assert!(ValidateUserNameService::run("test_user-123").is_ok());
    assert!(ValidateUserNameService::run("TestUser").is_ok());
    assert!(ValidateUserNameService::run("user123").is_ok());
    assert!(ValidateUserNameService::run("test-user").is_ok());
    assert!(ValidateUserNameService::run("test_user").is_ok());
  }
}
