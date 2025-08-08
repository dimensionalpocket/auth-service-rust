use dp_auth_service::database::Database;
use sqlx::SqlitePool;
use std::env;

async fn generate_schema_dump(pool: &SqlitePool) -> Result<String, sqlx::Error> {
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
  schema_sql.push_str("-- Generated automatically by migrate_and_dump.rs\n\n");

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
  // Load environment variables from .env file
  dotenvy::dotenv().ok();

  println!("Starting database migration, seeding, and schema dump...");

  // Get database URL from environment
  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  // Initialize database and run migrations
  let database = Database::new(&database_url).await?;
  database.migrate().await?;
  println!("✅ Migrations completed successfully");

  // Run seeds
  database.seed().await?;
  println!("✅ Seeds completed successfully");

  // Get database URL for schema dump
  let _database_url =
    env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/development.db".to_string());

  // Generate schema dump using pure Rust/sqlx instead of sqlite3 command
  let schema_sql = generate_schema_dump(&database.pool).await?;

  // Write schema to file
  std::fs::create_dir_all("config/database")?;
  std::fs::write("config/database/schema.sql", schema_sql)?;
  println!("✅ Schema dumped to config/database/schema.sql");

  println!("🎉 Migration, seeding, and schema dump completed successfully!");
  Ok(())
}
