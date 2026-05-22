use dps_config::DpsConfig;

pub struct GenerateSessionCookieService;

impl GenerateSessionCookieService {
  pub fn run(config: &DpsConfig, session_token: &str) -> String {
    let cookie_domain = format!(".{}", config.get_domain());
    let api_path = format!("/{}", config.get_api_path());
    format!(
      "{}={}; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Max-Age={}",
      crate::middleware::session::SESSION_COOKIE_NAME,
      session_token,
      cookie_domain,
      api_path,
      if config.get_auth_api_insecure_cookie() {
        ""
      } else {
        "; Secure"
      },
      config.get_auth_api_session_ttl_seconds()
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::create_test_dps_config;

  #[test]
  fn test_generate_session_cookie_secure() {
    let mut config = create_test_dps_config();
    config.set_domain("example.com");
    config.set_auth_api_insecure_cookie(false);
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
    let mut config = create_test_dps_config();
    config.set_domain("example.com");
    config.set_auth_api_insecure_cookie(true);

    let cookie = GenerateSessionCookieService::run(&config, "test-token");

    assert!(cookie.contains("DpsAuthSession=test-token;"));
    assert!(!cookie.contains("Secure"));
  }
}
