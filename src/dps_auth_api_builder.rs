use crate::dps_auth_api::{DpsAuthApi, DpsAuthApiError, ResolvedServerConfig};

#[derive(Debug, Default)]
pub struct DpsAuthApiBuilder {
  port: Option<u16>,
  sqlite_file_path: Option<String>,
  session_secret: Option<Vec<u8>>,
  cookie_domain: Option<String>,
  insecure_cookie: Option<bool>,
  development_mode: Option<bool>,
  database_pool_size: Option<u32>,
}

impl DpsAuthApiBuilder {
  pub fn port(mut self, port: u16) -> Self {
    self.port = Some(port);
    self
  }

  pub fn sqlite_file_path<S: Into<String>>(mut self, path: S) -> Self {
    self.sqlite_file_path = Some(path.into());
    self
  }

  pub fn session_secret(mut self, secret: Vec<u8>) -> Self {
    self.session_secret = Some(secret);
    self
  }

  pub fn cookie_domain<S: Into<String>>(mut self, domain: S) -> Self {
    self.cookie_domain = Some(domain.into());
    self
  }

  pub fn insecure_cookie(mut self, insecure: bool) -> Self {
    self.insecure_cookie = Some(insecure);
    self
  }

  pub fn development_mode(mut self, dev_mode: bool) -> Self {
    self.development_mode = Some(dev_mode);
    self
  }

  pub fn database_pool_size(mut self, size: u32) -> Self {
    self.database_pool_size = Some(size);
    self
  }

  pub fn build(self) -> Result<DpsAuthApi, DpsAuthApiError> {
    // Validate and resolve configuration with defaults
    let config = ResolvedServerConfig {
      port: self.port.unwrap_or(3000),
      sqlite_file_path: self
        .sqlite_file_path
        .unwrap_or_else(|| "data/development.db".to_string()),
      session_secret: self
        .session_secret
        .ok_or(DpsAuthApiError::MissingRequiredConfig {
          field: "session_secret".to_string(),
        })?,
      cookie_domain: self
        .cookie_domain
        .unwrap_or_else(|| ".api.dps.localhost".to_string()),
      insecure_cookie: self.insecure_cookie.unwrap_or(false),
      development_mode: self.development_mode.unwrap_or(false),
      database_pool_size: self.database_pool_size,
    };

    // Validate session secret length
    if config.session_secret.len() != 32 {
      return Err(DpsAuthApiError::InvalidSecretLength {
        actual: config.session_secret.len(),
        expected: 32,
      });
    }

    Ok(DpsAuthApi { config })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_builder_port() {
    let builder = DpsAuthApiBuilder::default().port(8080);
    assert_eq!(builder.port, Some(8080));
  }

  #[test]
  fn test_builder_sqlite_file_path() {
    let builder = DpsAuthApiBuilder::default().sqlite_file_path("test.db");
    assert_eq!(builder.sqlite_file_path, Some("test.db".to_string()));
  }

  #[test]
  fn test_builder_session_secret() {
    let secret = vec![1u8; 32];
    let builder = DpsAuthApiBuilder::default().session_secret(secret.clone());
    assert_eq!(builder.session_secret, Some(secret));
  }

  #[test]
  fn test_builder_cookie_domain() {
    let builder = DpsAuthApiBuilder::default().cookie_domain(".example.com");
    assert_eq!(builder.cookie_domain, Some(".example.com".to_string()));
  }

  #[test]
  fn test_builder_insecure_cookie() {
    let builder = DpsAuthApiBuilder::default().insecure_cookie(false);
    assert_eq!(builder.insecure_cookie, Some(false));
  }

  #[test]
  fn test_builder_development_mode() {
    let builder = DpsAuthApiBuilder::default().development_mode(false);
    assert_eq!(builder.development_mode, Some(false));
  }

  #[test]
  fn test_builder_chaining() {
    let secret = vec![1u8; 32];
    let builder = DpsAuthApiBuilder::default()
      .port(3000)
      .sqlite_file_path("data/test.db")
      .session_secret(secret.clone())
      .cookie_domain(".test.com")
      .insecure_cookie(true)
      .development_mode(true);

    assert_eq!(builder.port, Some(3000));
    assert_eq!(builder.sqlite_file_path, Some("data/test.db".to_string()));
    assert_eq!(builder.session_secret, Some(secret));
    assert_eq!(builder.cookie_domain, Some(".test.com".to_string()));
    assert_eq!(builder.insecure_cookie, Some(true));
    assert_eq!(builder.development_mode, Some(true));
  }

  #[test]
  fn test_builder_default() {
    let builder = DpsAuthApiBuilder::default();
    assert_eq!(builder.port, None);
    assert_eq!(builder.sqlite_file_path, None);
    assert_eq!(builder.session_secret, None);
    assert_eq!(builder.cookie_domain, None);
    assert_eq!(builder.insecure_cookie, None);
    assert_eq!(builder.development_mode, None);
  }

  #[test]
  fn test_builder_string_conversion() {
    let builder = DpsAuthApiBuilder::default()
      .sqlite_file_path(String::from("test.db"))
      .cookie_domain(String::from(".example.com"));

    assert_eq!(builder.sqlite_file_path, Some("test.db".to_string()));
    assert_eq!(builder.cookie_domain, Some(".example.com".to_string()));
  }
}

// Add tests for build() method
#[cfg(test)]
mod build_tests {
  use super::*;

  #[test]
  fn test_build_with_valid_config() {
    let secret = vec![1u8; 32];
    let server = DpsAuthApiBuilder::default()
      .session_secret(secret.clone())
      .build()
      .unwrap();

    // Should succeed with only session_secret specified
    assert_eq!(server.config.session_secret, secret);
    // All other fields should have correct defaults
    assert_eq!(server.config.port, 3000);
    assert_eq!(server.config.sqlite_file_path, "data/development.db");
    assert_eq!(server.config.cookie_domain, ".api.dps.localhost");
    assert!(!server.config.insecure_cookie);
    assert!(!server.config.development_mode);
  }

  #[test]
  fn test_build_missing_session_secret() {
    let result = DpsAuthApiBuilder::default().build();

    assert!(
      matches!(result, Err(DpsAuthApiError::MissingRequiredConfig { field }) if field == "session_secret")
    );
  }

  #[test]
  fn test_build_invalid_secret_length() {
    let secret = vec![1u8; 16]; // Wrong length
    let result = DpsAuthApiBuilder::default()
      .session_secret(secret)
      .build();

    assert!(matches!(
      result,
      Err(DpsAuthApiError::InvalidSecretLength {
        actual: 16,
        expected: 32
      })
    ));
  }

  #[test]
  fn test_build_overrides_defaults() {
    let secret = vec![2u8; 32];
    let server = DpsAuthApiBuilder::default()
      .port(8080)
      .sqlite_file_path("custom/path.db")
      .session_secret(secret.clone())
      .cookie_domain(".custom.com")
      .insecure_cookie(true)
      .development_mode(true)
      .build()
      .unwrap();

    assert_eq!(server.config.port, 8080);
    assert_eq!(server.config.sqlite_file_path, "custom/path.db");
    assert_eq!(server.config.session_secret, secret);
    assert_eq!(server.config.cookie_domain, ".custom.com");
    assert!(server.config.insecure_cookie);
    assert!(server.config.development_mode);
  }

  #[test]
  fn test_build_invalid_secret_lengths() {
    // Test various invalid lengths
    let test_cases = vec![0, 1, 15, 16, 31, 33, 64];

    for length in test_cases {
      let secret = vec![1u8; length];
      let result = DpsAuthApiBuilder::default()
        .session_secret(secret)
        .build();

      assert!(
        matches!(result, Err(DpsAuthApiError::InvalidSecretLength { actual, expected: 32 }) if actual == length)
      );
    }
  }
}

// TODO: add tests for database_pool_size
