use crate::types::SiteError;

pub struct ValidateSiteSlugService;

impl ValidateSiteSlugService {
  pub fn run(slug: &str) -> Result<(), SiteError> {
    if slug.trim().is_empty() {
      return Err(SiteError::ValidationError(
        "Slug cannot be empty".to_string(),
      ));
    }

    if slug.len() < 3 {
      return Err(SiteError::ValidationError(
        "Slug must be at least 3 characters long".to_string(),
      ));
    }

    if slug.len() > 20 {
      return Err(SiteError::ValidationError(
        "Slug cannot be longer than 20 characters".to_string(),
      ));
    }

    // Must start with a letter
    if !slug.chars().next().unwrap().is_ascii_alphabetic() {
      return Err(SiteError::ValidationError(
        "Slug must start with a letter".to_string(),
      ));
    }

    // Must end with a letter or number
    if !slug.chars().last().unwrap().is_ascii_alphanumeric() {
      return Err(SiteError::ValidationError(
        "Slug must end with a letter or number".to_string(),
      ));
    }

    // Check for valid characters (alphanumeric, underscore, hyphen)
    if !slug
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
      return Err(SiteError::ValidationError(
        "Slug can only contain letters, numbers, underscores, and hyphens".to_string(),
      ));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  #[dps_auth_db_test]
  async fn test_validate_slug_empty() {
    let result = ValidateSiteSlugService::run("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[dps_auth_db_test]
  async fn test_validate_slug_whitespace_only() {
    let result = ValidateSiteSlugService::run("   ");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[dps_auth_db_test]
  async fn test_validate_slug_too_short() {
    let result = ValidateSiteSlugService::run("ab");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("at least 3 characters"));
  }

  #[dps_auth_db_test]
  async fn test_validate_slug_too_long() {
    let result = ValidateSiteSlugService::run(&"a".repeat(21));
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("longer than 20 characters"));
  }

  #[dps_auth_db_test]
  async fn test_validate_slug_invalid_start() {
    let result = ValidateSiteSlugService::run("123example");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("must start with a letter"));
  }

  #[dps_auth_db_test]
  async fn test_validate_slug_invalid_end() {
    let result = ValidateSiteSlugService::run("example-");
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("must end with a letter or number"));
  }

  #[dps_auth_db_test]
  async fn test_validate_slug_invalid_characters() {
    let result = ValidateSiteSlugService::run("test@example");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("can only contain"));
  }

  #[dps_auth_db_test]
  async fn test_validate_slug_valid_cases() {
    assert!(ValidateSiteSlugService::run("example").is_ok());
    assert!(ValidateSiteSlugService::run("test-site-123").is_ok());
    assert!(ValidateSiteSlugService::run("my_site").is_ok());
    assert!(ValidateSiteSlugService::run("abc").is_ok());
    assert!(ValidateSiteSlugService::run("z9x").is_ok());
  }
}
