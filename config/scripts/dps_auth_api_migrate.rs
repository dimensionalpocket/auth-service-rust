use clap::Parser;
use dps_auth_api::{
  migration_config::{CliArgs, MigrationConfig},
  Database,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Parse command-line arguments
  let cli_args = CliArgs::parse();

  // Load and merge configuration from all sources
  let config = MigrationConfig::from_sources(cli_args)?;

  println!("🚀 Starting dps-auth-api database migration...");
  println!("📊 Configuration:");
  println!("   SQLite file: {}", config.sqlite_file.display());
  println!("   Skip seeds: {}", config.skip_seeds);
  println!();

  // Initialize database
  let database = Database::new(&config.sqlite_file.display().to_string()).await?;

  // Run migrations (always use library's internal migrations)
  database.migrate().await?;
  println!("✅ Migrations completed successfully");

  // Run seeds (if not skipped)
  if !config.skip_seeds {
    database.seed().await?;
    println!("✅ Seeds completed successfully");
  } else {
    println!("⏭️  Skipping seeds (disabled by configuration)");
  }

  // Generate schema dump (always to library's internal location)
  database
    .dump_schema_to_file("config/database/schema.sql")
    .await?;

  println!("🎉 dps-auth-api database migration completed successfully!");
  Ok(())
}
