use sqlx::migrate::Migrator;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Row, Sqlite, SqlitePool};
use std::collections::HashSet;
use std::fs;

pub struct SqliteDatabaseCore {
  pub pool: SqlitePool,
}

impl SqliteDatabaseCore {
  pub async fn new_with_pool_size(
    sqlite_file_path: &str,
    pool_size: Option<u32>,
    pragma_commands: &'static [&'static str],
  ) -> Result<Self, sqlx::Error> {
    let database_url = format!("sqlite:{sqlite_file_path}");

    if !Sqlite::database_exists(&database_url)
      .await
      .unwrap_or(false)
    {
      Sqlite::create_database(&database_url).await?;
    }

    let pool = match pool_size {
      Some(size) => {
        SqlitePoolOptions::new()
          .max_connections(size)
          .acquire_timeout(std::time::Duration::from_secs(2))
          .connect(&database_url)
          .await?
      }
      None => SqlitePool::connect(&database_url).await?,
    };

    Self::configure_sqlite(&pool, pragma_commands).await?;

    Ok(Self { pool })
  }

  pub async fn configure_sqlite(
    pool: &SqlitePool,
    pragma_commands: &'static [&'static str],
  ) -> Result<(), sqlx::Error> {
    for command in pragma_commands.iter() {
      sqlx::query(command).execute(pool).await?;
    }

    Self::verify_sqlite_config(pool).await?;
    Ok(())
  }

  async fn verify_sqlite_config(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let journal_mode: String = sqlx::query("PRAGMA journal_mode;")
      .fetch_one(pool)
      .await?
      .get(0);

    if journal_mode.to_uppercase() != "WAL" {
      eprintln!("⚠️  Warning: WAL mode not enabled, got: {journal_mode}");
    }

    let foreign_keys: i64 = sqlx::query("PRAGMA foreign_keys;")
      .fetch_one(pool)
      .await?
      .get(0);

    if foreign_keys != 1 {
      eprintln!("⚠️  Warning: Foreign keys not enabled");
    }

    Ok(())
  }

  pub async fn migrate(
    pool: &SqlitePool,
    migrator: &'static Migrator,
  ) -> Result<(), sqlx::migrate::MigrateError> {
    let applied = sqlx::query("SELECT version FROM _sqlx_migrations WHERE success = true")
      .fetch_all(pool)
      .await
      .unwrap_or_else(|_| Vec::new());
    let applied_versions: HashSet<_> = applied
      .iter()
      .map(|row| row.get::<i64, _>("version"))
      .collect();

    let mut pending_count = 0;
    let mut seen_versions = HashSet::new();
    for migration in migrator.iter() {
      if !applied_versions.contains(&migration.version)
        && !seen_versions.contains(&migration.version)
      {
        println!(
          "Pending migration: {} - {}",
          migration.version, migration.description
        );
        seen_versions.insert(migration.version);
        pending_count += 1;
      }
    }

    if pending_count == 0 {
      println!("No pending migrations found");
      return Ok(());
    }

    println!("Applying {pending_count} pending migration(s)...");
    migrator.run(pool).await?;
    println!("Successfully applied {pending_count} migration(s)");
    Ok(())
  }

  pub async fn revert(
    pool: &SqlitePool,
    migrator: &'static Migrator,
    steps: usize,
  ) -> Result<usize, sqlx::migrate::MigrateError> {
    // Get list of applied migrations (checking both success = true and success = 1 for compatibility)
    let applied =
      sqlx::query("SELECT version FROM _sqlx_migrations WHERE success != 0 ORDER BY version ASC")
        .fetch_all(pool)
        .await
        .unwrap_or_else(|_| Vec::new());

    if applied.is_empty() {
      println!("No migrations to revert");
      return Ok(0);
    }

    let applied_count = applied.len();
    let revert_count = steps.min(applied_count);

    println!("Found {applied_count} applied migration(s)");
    println!("Reverting {revert_count} migration(s)...");

    // Calculate target version
    // If we want to revert N migrations, we need to go back to the version
    // that is (applied_count - revert_count) from the start
    // If reverting all migrations, target is 0 (empty database)
    let target_version = if revert_count >= applied_count {
      0 // Revert all migrations
    } else {
      // Get the version to revert TO (the one that should remain applied)
      let target_index = applied_count - revert_count - 1;
      applied[target_index].get::<i64, _>("version")
    };

    println!("Target version: {target_version}");

    migrator.undo(pool, target_version).await?;

    println!("Successfully reverted {revert_count} migration(s)");
    Ok(revert_count)
  }

  pub async fn seed(
    pool: &SqlitePool,
    seeds_dir: &'static str,
  ) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(seeds_dir).exists() {
      println!("No seeds directory found, skipping seeding");
      return Ok(());
    }

    let mut seed_files = fs::read_dir(seeds_dir)?
      .filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension()? == "sql" {
          Some(path)
        } else {
          None
        }
      })
      .collect::<Vec<_>>();

    seed_files.sort();

    for seed_file in seed_files {
      let sql_content = fs::read_to_string(&seed_file)?;
      println!("Running seed: {}", seed_file.display());

      for (index, statement) in sql_content.split(';').enumerate() {
        let statement = statement.trim();

        if statement.is_empty() {
          continue;
        }

        let sql_lines: Vec<&str> = statement
          .lines()
          .filter(|line| !line.trim().is_empty() && !line.trim().starts_with("--"))
          .collect();

        if sql_lines.is_empty() {
          continue;
        }

        let clean_sql = sql_lines.join("\n").trim().to_string();

        match sqlx::query(&clean_sql).execute(pool).await {
          Ok(result) => {
            println!(
              "  ✅ Statement {} executed successfully, rows affected: {}",
              index + 1,
              result.rows_affected()
            );
          }
          Err(e) => {
            eprintln!(
              "❌ Error executing statement {} in seed file: {}",
              index + 1,
              seed_file.display()
            );
            eprintln!("Error: {e}");
            eprintln!("SQL that failed:");
            eprintln!("--- START SQL ---");
            eprintln!("{clean_sql}");
            eprintln!("--- END SQL ---");
            return Err(Box::new(e));
          }
        }
      }
    }

    println!("✅ Seeds completed successfully");
    Ok(())
  }

  pub async fn dump_schema_content(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let tables = sqlx::query_scalar::<_, String>(
      "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    let indexes = sqlx::query_scalar::<_, String>(
      "SELECT sql FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' AND sql IS NOT NULL ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    let mut schema_sql = String::new();
    schema_sql.push_str("-- Database Schema Dump\n");
    schema_sql.push_str("-- Generated automatically by dps-auth-api-migrate\n\n");

    for table_sql in tables {
      schema_sql.push_str(&table_sql);
      schema_sql.push_str(";\n\n");
    }

    for index_sql in indexes {
      schema_sql.push_str(&index_sql);
      schema_sql.push_str(";\n\n");
    }

    Ok(schema_sql)
  }

  pub async fn dump_schema_to_file(
    pool: &SqlitePool,
    file_path: &str,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let schema_sql = Self::dump_schema_content(pool).await?;

    if let Some(parent) = std::path::Path::new(file_path).parent() {
      std::fs::create_dir_all(parent)?;
    }

    std::fs::write(file_path, schema_sql)?;
    println!("✅ Schema dumped to {file_path}");
    Ok(())
  }
}
