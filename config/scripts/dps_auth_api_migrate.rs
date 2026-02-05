use clap::Parser;
use dps_auth_api::migration_config::{CliArgs, MigrationConfig};
use dps_auth_api::MainDatabase;

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
  if let Some(revert_steps) = config.revert {
    println!("   Revert steps: {revert_steps}");
  }
  println!();

  // Initialize database
  let main_database = MainDatabase::new(&config.sqlite_file.display().to_string()).await?;

  // Handle revert mode
  if let Some(steps) = config.revert {
    println!("⏮️  Revert mode enabled");
    main_database.revert(steps).await?;

    // Regenerate schema dump after revert
    main_database
      .dump_schema_to_file("config/database/schema.sql")
      .await?;

    println!("🎉 Migration revert completed successfully!");
    return Ok(());
  }

  // Run migrations (always use library's internal migrations)
  main_database.migrate().await?;
  println!("✅ Migrations completed successfully");

  // Run seeds (if not skipped)
  if !config.skip_seeds {
    main_database.seed().await?;
    println!("✅ Seeds completed successfully");
  } else {
    println!("⏭️  Skipping seeds (disabled by configuration)");
  }

  // Generate schema dump (always to library's internal location)
  main_database
    .dump_schema_to_file("config/database/schema.sql")
    .await?;

  println!("🎉 dps-auth-api database migration completed successfully!");
  Ok(())
}
