use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dps-auth-api-migrate")]
#[command(about = "Database migration and setup tool for dps-auth-api")]
#[command(version)]
pub struct CliArgs {
  /// Path to SQLite database file
  #[arg(long, env = "DPS_AUTH_API_SQLITE_FILE")]
  pub sqlite_file: Option<PathBuf>,

  /// Skip running seed files
  #[arg(long, env = "DPS_AUTH_API_MIGRATE_SKIP_SEEDS")]
  pub skip_seeds: bool,

  /// Revert the last N migrations (default: 1)
  #[arg(long, value_name = "N", default_missing_value = "1")]
  pub revert: Option<usize>,
}

#[derive(Debug)]
pub struct MigrationConfig {
  pub sqlite_file: PathBuf,
  pub skip_seeds: bool,
  pub revert: Option<usize>,
}

impl MigrationConfig {
  /// Merge configuration from CLI (which already reads environment variables via clap)
  /// and fall back to sensible defaults.
  pub fn from_sources(cli_args: CliArgs) -> Result<Self, Box<dyn std::error::Error>> {
    let sqlite_file = cli_args
      .sqlite_file
      .unwrap_or_else(|| PathBuf::from("data/main-development.db"));

    let skip_seeds = cli_args.skip_seeds;
    let revert = cli_args.revert;

    Ok(MigrationConfig {
      sqlite_file,
      skip_seeds,
      revert,
    })
  }
}
