use clap::Parser;
use dp_auth_service::{
  migration_config::{CliArgs, MigrationConfig},
  Database,
};
use sqlx::SqlitePool;

async fn generate_schema_dump(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
  // Generate schema dump using pure Rust/sqlx instead of sqlite3 command
  let schema_sql = generate_schema_dump_content(pool).await?;

  // Write schema to library's internal location
  std::fs::create_dir_all("config/database")?;
  std::fs::write("config/database/schema.sql", schema_sql)?;
  println!("✅ Schema dumped to config/database/schema.sql");

  Ok(())
}

async fn generate_schema_dump_content(pool: &SqlitePool) -> Result<String, sqlx::Error> {
  // Query to get all table creation statements
  let tables = sqlx::query_scalar::<_, String>(
    "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
  )
  .fetch_all(pool)
  .await?;

  // Query to get all index creation statements
  let indexes = sqlx::query_scalar::<_, String>(
        "SELECT sql FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' AND sql IS NOT NULL ORDER BY name"
    )
    .fetch_all(pool)
    .await?;

  // Combine all SQL statements
  let mut schema_sql = String::new();
  schema_sql.push_str("-- Database Schema Dump\n");
  schema_sql.push_str("-- Generated automatically by dp-auth-migrate\n\n");

  // Add table creation statements
  for table_sql in tables {
    schema_sql.push_str(&table_sql);
    schema_sql.push_str(";\n\n");
  }

  // Add index creation statements
  for index_sql in indexes {
    schema_sql.push_str(&index_sql);
    schema_sql.push_str(";\n\n");
  }

  Ok(schema_sql)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Parse command-line arguments
  let cli_args = CliArgs::parse();

  // Load and merge configuration from all sources
  let config = MigrationConfig::from_sources(cli_args)?;

  println!("🚀 Starting dp-auth database migration...");
  println!("📊 Configuration:");
  println!("   SQLite file: {}", config.sqlite_file.display());
  println!("   Database URL: {}", config.database_url());
  println!("   Skip seeds: {}", config.skip_seeds);
  println!();

  // Initialize database
  let database = Database::new(&config.database_url()).await?;

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
  generate_schema_dump(&database.pool).await?;

  println!("🎉 dp-auth database migration completed successfully!");
  Ok(())
}
