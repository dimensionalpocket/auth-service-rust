#[derive(Debug, Clone)]
pub struct ServerConfig {
  pub port: u16,
  pub database_url: String,
  pub session_secret: Vec<u8>, // 32-byte secret
  pub cookie_domain: String,
  pub insecure_cookie: bool,
  pub development_mode: bool,
}

impl ServerConfig {
  pub fn new(
    port: u16,
    database_url: String,
    session_secret: Vec<u8>,
    cookie_domain: String,
    insecure_cookie: bool,
    development_mode: bool,
  ) -> Result<Self, ConfigError> {
    // Validate session secret is exactly 32 bytes
    if session_secret.len() != 32 {
      return Err(ConfigError::InvalidSecretLength {
        actual: session_secret.len(),
        expected: 32,
      });
    }

    Ok(Self {
      port,
      database_url,
      session_secret,
      cookie_domain,
      insecure_cookie,
      development_mode,
    })
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
  InvalidSecretLength { actual: usize, expected: usize },
}

impl std::fmt::Display for ConfigError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ConfigError::InvalidSecretLength { actual, expected } => {
        write!(
          f,
          "Invalid secret length: got {actual} bytes, expected {expected}"
        )
      }
    }
  }
}

impl std::error::Error for ConfigError {}
