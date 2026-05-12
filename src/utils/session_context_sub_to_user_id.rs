use crate::middleware::session::SessionContext;
use crate::types::DpsAuthApiConfig;

/// Extracts the user ID from a session context by parsing the string `sub` field.
///
/// # Arguments
/// * `session_context` - The session context containing the payload
/// * `_config` - The API config (reserved for future use, e.g., validation rules)
///
/// # Returns
/// * `Ok(i64)` - The parsed user ID
/// * `Err(String)` - Error message if session is missing or sub cannot be parsed
pub fn session_context_sub_to_user_id(
  session_context: &SessionContext,
  _config: &DpsAuthApiConfig,
) -> Result<i64, String> {
  let payload = session_context
    .payload
    .as_ref()
    .ok_or_else(|| "No valid session".to_string())?;

  payload
    .sub
    .parse::<i64>()
    .map_err(|e| format!("Invalid user ID in session: {e}"))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionPayload;

  fn create_test_config() -> DpsAuthApiConfig {
    DpsAuthApiConfig {
      port: 3000,
      sqlite_main_file_path: ":memory:".to_string(),
      sqlite_session_file_path: ":memory:".to_string(),
      session_secret: vec![0u8; 32],
      cookie_domain: ".test.com".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: true,
      development_mode: true,
      sqlite_main_pool_size: 1,
      sqlite_session_pool_size: 1,
      session_ttl_seconds: 3600,
    }
  }

  #[test]
  fn test_valid_session_returns_user_id() {
    let payload = SessionPayload {
      sub: "123".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(payload));
    let config = create_test_config();

    let result = session_context_sub_to_user_id(&session_context, &config);
    assert_eq!(result, Ok(123));
  }

  #[test]
  fn test_no_session_returns_error() {
    let session_context = SessionContext::new(None);
    let config = create_test_config();

    let result = session_context_sub_to_user_id(&session_context, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No valid session"));
  }

  #[test]
  fn test_invalid_sub_returns_error() {
    let payload = SessionPayload {
      sub: "not_a_number".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(payload));
    let config = create_test_config();

    let result = session_context_sub_to_user_id(&session_context, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid user ID"));
  }
}
