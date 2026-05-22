use dps_auth_api::DpsAuthApi;
use dps_config::DpsConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file for local development
  dotenvy::dotenv().ok();

  // Print DPS_ environment variables for debugging
  println!("=== DPS Configuration Variables ===");
  for (key, value) in std::env::vars() {
    if key.starts_with("DPS_") {
      println!("{key}: {value}");
    }
  }
  println!("====================================\n");

  // Initialize logging
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "dps_auth_api=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  // DpsConfig::new() automatically loads from environment variables
  let config = DpsConfig::new();
  let server = DpsAuthApi::new(config)?;
  Ok(server.start().await?)
}
