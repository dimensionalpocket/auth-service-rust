#[derive(Debug, Clone)]
pub struct DpsAuthApiConfig {
  pub port: u16,
  pub sqlite_main_file_path: String,
  pub sqlite_session_file_path: String,
  pub session_secret: Vec<u8>,
  pub cookie_domain: String,
  pub api_path: String,
  pub insecure_cookie: bool,
  pub development_mode: bool,
  pub sqlite_main_pool_size: u16,
  pub sqlite_session_pool_size: u16,
  pub session_ttl_seconds: u32,
}
