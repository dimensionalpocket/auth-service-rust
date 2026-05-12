use crate::types::DpsAuthApiConfig;

pub struct GenerateLogoutCookieService;

impl GenerateLogoutCookieService {
  pub fn run(config: &DpsAuthApiConfig) -> String {
    format!(
      "{}=; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
      crate::middleware::session::SESSION_COOKIE_NAME,
      config.cookie_domain,
      config.api_path,
      if config.insecure_cookie {
        ""
      } else {
        "; Secure"
      }
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
  fn test_generate_logout_cookie_secure() {
    let config = create_test_config();
    let cookie = GenerateLogoutCookieService::run(&config);

    assert!(cookie.starts_with("DpsAuthSession=;"));
    assert!(cookie.contains("Domain=.example.com"));
    assert!(cookie.contains("Path=/api"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("Expires=Thu, 01 Jan 1970 00:00:00 GMT"));
  }

  #[test]
  fn test_generate_logout_cookie_insecure() {
    let mut config = create_test_config();
    config.insecure_cookie = true;

    let cookie = GenerateLogoutCookieService::run(&config);

    assert!(!cookie.contains("Secure"));
  }
}
