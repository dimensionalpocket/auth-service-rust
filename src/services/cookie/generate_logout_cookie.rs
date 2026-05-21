use dps_config::DpsConfig;

pub struct GenerateLogoutCookieService;

impl GenerateLogoutCookieService {
  pub fn run(config: &DpsConfig) -> String {
    let cookie_domain = format!(".{}", config.get_domain());
    let api_path = format!("/{}", config.get_api_path());
    format!(
      "{}=; Domain={}; Path={}; HttpOnly; SameSite=Lax{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
      crate::middleware::session::SESSION_COOKIE_NAME,
      cookie_domain,
      api_path,
      if config.get_auth_api_insecure_cookie() {
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
  use crate::test_utils::create_test_dps_config;

  #[test]
  fn test_generate_logout_cookie_secure() {
    let mut config = create_test_dps_config();
    config.set_domain("example.com");
    config.set_auth_api_insecure_cookie(false);
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
    let mut config = create_test_dps_config();
    config.set_domain("example.com");
    config.set_auth_api_insecure_cookie(true);

    let cookie = GenerateLogoutCookieService::run(&config);

    assert!(!cookie.contains("Secure"));
  }
}
