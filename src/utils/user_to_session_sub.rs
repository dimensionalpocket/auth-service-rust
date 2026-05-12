use crate::models::User;
use crate::types::DpsAuthApiConfig;

/// Converts a user to the string `sub` value used in session payloads.
///
/// # Arguments
/// * `user` - The user to convert
/// * `_config` - The API config (reserved for future use)
///
/// # Returns
/// The string representation of the user ID for use as the session `sub`
pub fn user_to_session_sub(user: &User, _config: &DpsAuthApiConfig) -> String {
  user.id.to_string()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::User;

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

  fn create_test_user(id: i64) -> User {
    User {
      id,
      uuid: "test-uuid".to_string(),
      created_ts: 1000,
      updated_ts: 1000,
      name: "testuser".to_string(),
      role_id: 1,
      password_hash: "hash".to_string(),
      metadata_json: None,
    }
  }

  #[test]
  fn test_user_to_session_sub_returns_id_as_string() {
    let user = create_test_user(42);
    let config = create_test_config();

    let result = user_to_session_sub(&user, &config);
    assert_eq!(result, "42");
  }

  #[test]
  fn test_user_to_session_sub_zero_id() {
    let user = create_test_user(0);
    let config = create_test_config();

    let result = user_to_session_sub(&user, &config);
    assert_eq!(result, "0");
  }

  #[test]
  fn test_user_to_session_sub_large_id() {
    let user = create_test_user(i64::MAX);
    let config = create_test_config();

    let result = user_to_session_sub(&user, &config);
    assert_eq!(result, i64::MAX.to_string());
  }
}
