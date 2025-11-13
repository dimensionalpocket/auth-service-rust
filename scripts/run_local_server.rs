use dps_auth_api::utils::get_secret_from_env::get_secret_from_env;
use dps_auth_api::DpsAuthApi;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file for local development
  dotenvy::dotenv().ok();

  // Initialize logging
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "dps_auth_api=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  // Read configuration from environment variables
  let port = env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string())
    .parse::<u16>()
    .expect("PORT must be a valid number");

  let sqlite_file_path =
    env::var("DPS_AUTH_SQLITE_FILE").unwrap_or_else(|_| "data/development.db".to_string());

  let session_secret = get_secret_from_env("DPS_AUTH_SECRET_KEY", 32)
    .expect("DPS_AUTH_SECRET_KEY environment variable is required");

  let cookie_domain =
    env::var("DPS_AUTH_COOKIE_DOMAIN").unwrap_or_else(|_| ".api.dps.localhost".to_string());

  let insecure_cookie = env::var("DPS_AUTH_INSECURE_COOKIE").is_ok();

  let development_mode = env::var("DPS_AUTH_ENV").unwrap_or_default() == "development";

  let server = DpsAuthApi::new()
    .port(port)
    .sqlite_file_path(sqlite_file_path)
    .session_secret(session_secret)
    .cookie_domain(cookie_domain)
    .insecure_cookie(insecure_cookie)
    .development_mode(development_mode)
    .build()?;

  Ok(server.start().await?)
}
