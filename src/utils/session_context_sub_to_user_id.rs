use crate::middleware::session::SessionContext;
use dps_config::DpsConfig;

/// Extracts the user ID from a session context by parsing the string `sub` field.
///
/// # Arguments
/// * `session_context` - The session context containing the payload
/// * `config` - The API config containing the session_sub_to_user_id_fn
///
/// # Returns
/// * `Ok(i64)` - The parsed user ID
/// * `Err(String)` - Error message if session is missing or sub cannot be parsed
pub fn session_context_sub_to_user_id(
  session_context: &SessionContext,
  config: &DpsConfig,
) -> Result<i64, String> {
  let payload = session_context
    .payload
    .as_ref()
    .ok_or_else(|| "No valid session".to_string())?;

  let sub_to_user_id_fn = config.get_session_sub_to_user_id_fn();
  sub_to_user_id_fn(&payload.sub).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionPayload;
  use crate::test_utils::create_test_dps_config;

  #[test]
  fn test_valid_session_returns_user_id() {
    let payload = SessionPayload {
      sub: "123".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(payload));
    let config = create_test_dps_config();

    let result = session_context_sub_to_user_id(&session_context, &config);
    assert_eq!(result, Ok(123));
  }

  #[test]
  fn test_no_session_returns_error() {
    let session_context = SessionContext::new(None);
    let config = create_test_dps_config();

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
    let config = create_test_dps_config();

    let result = session_context_sub_to_user_id(&session_context, &config);
    assert!(result.is_err());
  }
}
