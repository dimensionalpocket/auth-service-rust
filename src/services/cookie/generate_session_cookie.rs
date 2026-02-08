use crate::dps_auth_api::DpsAuthApiConfig;

pub struct GenerateSessionCookieService;

impl GenerateSessionCookieService {
  pub fn run(config: &DpsAuthApiConfig, session_token: &str) -> String {
    format!(
      "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
      crate::middleware::session::SESSION_COOKIE_NAME,
      session_token,
      config.cookie_domain,
      config.api_path,
      if config.insecure_cookie {
        ""
      } else {
        "; Secure"
      },
      config.session_ttl_seconds
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn create_test_config() -> DpsAuthApiConfig {
    DpsAuthApiConfig {
      port: 8080,
      sqlite_main_file_path: "test.db".to_string(),
      sqlite_session_file_path: "test.db.session".to_string(),
      session_secret: vec![0u8; 32],
      cookie_domain: ".example.com".to_string(),
      api_path: "/api".to_string(),
      insecure_cookie: false,
      development_mode: false,
      sqlite_main_pool_size: 1,
      sqlite_session_pool_size: 1,
      session_ttl_seconds: 3600,
    }
  }

  #[test]
  fn test_generate_session_cookie_secure() {
    let config = create_test_config();
    let cookie = GenerateSessionCookieService::run(&config, "test-token");

    assert!(cookie.starts_with("DpsAuthSession=test-token;"));
    assert!(cookie.contains("Domain=.example.com"));
    assert!(cookie.contains("Path=/api"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("Max-Age=3600"));
  }

  #[test]
  fn test_generate_session_cookie_insecure() {
    let mut config = create_test_config();
    config.insecure_cookie = true;

    let cookie = GenerateSessionCookieService::run(&config, "test-token");

    assert!(cookie.contains("DpsAuthSession=test-token;"));
    assert!(!cookie.contains("Secure"));
  }
}
