use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dp-auth-migrate")]
#[command(about = "Database migration and setup tool for dp-auth-service")]
#[command(version)]
pub struct CliArgs {
  /// Path to SQLite database file
  #[arg(long, env = "DP_AUTH_SQLITE_FILE")]
  pub sqlite_file: Option<PathBuf>,

  /// Path to configuration file
  #[arg(long, env = "DP_AUTH_MIGRATE_CONFIG_FILE")]
  pub config: Option<PathBuf>,

  /// Skip running seed files
  #[arg(long, env = "DP_AUTH_MIGRATE_SKIP_SEEDS")]
  pub skip_seeds: bool,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ConfigFile {
  pub database: Option<DatabaseConfig>,
  pub options: Option<OptionsConfig>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DatabaseConfig {
  pub sqlite_file: Option<PathBuf>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OptionsConfig {
  pub skip_seeds: Option<bool>,
}

#[derive(Debug)]
pub struct MigrationConfig {
  pub sqlite_file: PathBuf,
  pub skip_seeds: bool,
}

impl MigrationConfig {
  pub fn from_sources(cli_args: CliArgs) -> Result<Self, Box<dyn std::error::Error>> {
    // Load config file if specified or if default exists
    let config_file = Self::load_config_file(&cli_args)?;

    // Merge configurations with priority: CLI > Config File > Env > Defaults
    let sqlite_file = cli_args
      .sqlite_file
      .or_else(|| {
        config_file
          .database
          .as_ref()
          .and_then(|d| d.sqlite_file.clone())
      })
      .unwrap_or_else(|| PathBuf::from("data/development.db"));

    let skip_seeds = cli_args.skip_seeds
      || config_file
        .options
        .as_ref()
        .and_then(|o| o.skip_seeds)
        .unwrap_or(false);

    Ok(MigrationConfig {
      sqlite_file,
      skip_seeds,
    })
  }

  pub fn database_url(&self) -> String {
    format!("sqlite:{}", self.sqlite_file.display())
  }

  fn load_config_file(cli_args: &CliArgs) -> Result<ConfigFile, Box<dyn std::error::Error>> {
    let config_path = cli_args
      .config
      .clone()
      .unwrap_or_else(|| PathBuf::from("./dp-auth-migrate.toml"));

    if config_path.exists() {
      let content = std::fs::read_to_string(&config_path)?;
      let config: ConfigFile = toml::from_str(&content)?;
      println!("✅ Loaded configuration from: {}", config_path.display());
      Ok(config)
    } else {
      Ok(ConfigFile::default())
    }
  }
}
