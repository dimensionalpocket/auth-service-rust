# Improve Migration Binary for Library Usage

**Date**: 2025-01-27@15:42  
**Status**: Draft  
**Priority**: Medium  

## Overview

Convert the `migrate_and_dump` binary to a more library-friendly `dp-auth-migrate` binary with configurable paths, multiple configuration methods, and smart defaults for library consumers.

## Current Issues

1. **Binary name**: `migrate_and_dump` is not descriptive for library consumers
2. **Database URL complexity**: Users need to specify "sqlite:" prefix manually
3. **Limited configuration**: Only supports environment variables
4. **Poor library integration**: Database functionality not exported from lib.rs
5. **Inflexible usage**: No command-line arguments or config file support
6. **Unnecessary schema configuration**: Users shouldn't need to configure schema dump location

## Goals

1. Rename binary to `dp-auth-migrate` for clarity
2. Add multiple configuration methods (CLI args, config file, env vars)
3. Simplify database file specification (remove need for "sqlite:" prefix)
4. Use library's internal migrations and seeds (not user-configurable)
5. Export database functionality from library
6. Improve documentation for library consumers
7. Handle schema dump internally without user configuration

## Configuration Methods (Priority Order)

The binary will support configuration through multiple methods, with the following priority order:

1. **Command-line arguments** (highest priority)
2. **Configuration file** (medium priority)
3. **Environment variables** (lowest priority)

### 1. Command-Line Arguments

```bash
dp-auth-migrate [OPTIONS]

OPTIONS:
    --sqlite-file <PATH>          Path to SQLite database file
    --config <PATH>               Path to configuration file
    --skip-seeds                  Skip running seed files
    --help                        Show help information
    --version                     Show version information
```

### 2. Configuration File

Support for a TOML configuration file (default: `dp-auth-migrate.toml`):

```toml
# dp-auth-migrate.toml
[database]
sqlite_file = "./auth.db"

[options]
skip_seeds = false
```

### 3. Environment Variables

Strictly named environment variables with `DP_AUTH_MIGRATE_` prefix:

```bash
DP_AUTH_SQLITE_FILE="data/development.db"
DP_AUTH_MIGRATE_SKIP_SEEDS="false"
DP_AUTH_MIGRATE_CONFIG_FILE="./dp-auth-migrate.toml"
```

## Smart Defaults

When used as a library dependency, the binary will use these defaults:

| Setting | Default Value | Rationale |
|---------|---------------|-----------|
| `sqlite_file` | `"data/development.db"` | Matches library's development database location |
| `config_file` | `"./dp-auth-migrate.toml"` | Standard config file name |
| `skip_seeds` | `false` | Run seeds by default |

### Internal Library Paths

The binary always uses the library's internal migrations and seeds:
- **Migrations**: Always use library's embedded migrations (`config/database/migrations`)
- **Seeds**: Always use library's embedded seeds (`config/database/seeds`)
- **Schema**: Always dump to library's schema location (`config/database/schema.sql`)

## Implementation Plan

### 1. Add Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
clap = { version = "4.0", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
```

### 2. Update Cargo.toml Binary Configuration

```toml
# Database migration and setup tool
[[bin]]
name = "dp-auth-migrate"
path = "config/scripts/dp_auth_migrate.rs"
```

### 3. Create Configuration Structures

Create `src/migration_config.rs`:
```rust
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dp-auth-migrate")]
#[command(about = "Database migration and setup tool for dp-auth-service")]
#[command(version)]
pub struct CliArgs {
    /// Path to SQLite database file
    #[arg(long, env = "DP_AUTH_MIGRATE_SQLITE_FILE")]
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
    pub paths: Option<PathsConfig>,
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
        let sqlite_file = cli_args.sqlite_file
            .or_else(|| config_file.database.as_ref().and_then(|d| d.sqlite_file.clone()))
            .unwrap_or_else(|| PathBuf::from("./auth.db"));

        let skip_seeds = cli_args.skip_seeds
            || config_file.options.as_ref().and_then(|o| o.skip_seeds).unwrap_or(false);

        Ok(MigrationConfig {
            sqlite_file,
            skip_seeds,
        })
    }
    
    pub fn database_url(&self) -> String {
        format!("sqlite:{}", self.sqlite_file.display())
    }

    fn load_config_file(cli_args: &CliArgs) -> Result<ConfigFile, Box<dyn std::error::Error>> {
        let config_path = cli_args.config.as_ref()
            .map(|p| p.clone())
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
```

### 4. Update Database Module

No changes needed to `src/database/mod.rs` - it already uses the correct internal paths:
- Migrations: `./config/database/migrations` 
- Seeds: `config/database/seeds`
- Schema: `config/database/schema.sql`

The existing `migrate()` and `seed()` methods are perfect as-is.

### 5. Create New Migration Binary

Create `config/scripts/dp_auth_migrate.rs`:

```rust
use clap::Parser;
use dp_auth_service::{Database, migration_config::{CliArgs, MigrationConfig}};
use sqlx::SqlitePool;

async fn generate_schema_dump(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // [Existing schema dump logic from original migrate_and_dump.rs]
    // Always write to config/database/schema.sql (library's internal location)
    
    let schema_sql = generate_schema_dump_content(pool).await?;
    
    // Write schema to library's internal location
    std::fs::create_dir_all("config/database")?;
    std::fs::write("config/database/schema.sql", schema_sql)?;
    println!("✅ Schema dumped to config/database/schema.sql");
    
    Ok(())
}

async fn generate_schema_dump_content(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    // [Copy existing logic from migrate_and_dump.rs]
    // ... (existing implementation)
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
```

### 6. Update Library Exports

Add to `src/lib.rs`:
```rust
pub mod migration_config;
pub use database::Database;
```

### 7. Update Documentation

Add the following section to `README.md`:

## Database Setup

### Using the Migration Binary

The library provides a `dp-auth-migrate` binary for database setup.

#### Building the Migration Binary

When you include this library as a dependency, you can build the migration binary:

    # Build the migration binary
    cargo build --bin dp-auth-migrate

    # Or build in release mode for production
    cargo build --release --bin dp-auth-migrate

The binary will be available at:
- Debug: `target/debug/dp-auth-migrate`
- Release: `target/release/dp-auth-migrate`

#### Running the Migration Binary

    # Basic usage with defaults (during development)
    cargo run --bin dp-auth-migrate

    # With custom SQLite file
    cargo run --bin dp-auth-migrate -- --sqlite-file ./my-auth.db

    # Using a configuration file
    cargo run --bin dp-auth-migrate --config ./my-config.toml

    # Skip seeds
    cargo run --bin dp-auth-migrate --skip-seeds

#### Production Deployment

For production deployments, build the binary in release mode and run it directly:

    # Build for production
    cargo build --release --bin dp-auth-migrate

    # Run the built binary
    ./target/release/dp-auth-migrate --sqlite-file ./production.db

    # Or with environment variables
    export DP_AUTH_MIGRATE_SQLITE_FILE="./production.db"
    export DP_AUTH_MIGRATE_SKIP_SEEDS="false"
    ./target/release/dp-auth-migrate

#### Docker Deployment

If using Docker, include the binary in your Dockerfile:

    # Build stage
    FROM rust:1.88.0 as builder
    WORKDIR /app
    COPY . .
    RUN cargo build --release --bin your-app --bin dp-auth-migrate

    # Runtime stage
    FROM debian:bullseye-slim
    WORKDIR /app
    COPY --from=builder /app/target/release/your-app /app/your-app
    COPY --from=builder /app/target/release/dp-auth-migrate /app/dp-auth-migrate

    # Run migrations before starting your app
    CMD ["sh", "-c", "./dp-auth-migrate && ./your-app"]
### Configuration Methods

1. **Configuration File** (`dp-auth-migrate.toml`):
```toml
[database]
sqlite_file = "./auth.db"

[options]
skip_seeds = false
```

2. **Environment Variables**:
```bash
export DP_AUTH_MIGRATE_SQLITE_FILE="./auth.db"
export DP_AUTH_MIGRATE_SKIP_SEEDS="false"
```

3. **Command-line Arguments** (see `--help` for full list)

### 8. Update Deployment Files

Update `Dockerfile`:
```dockerfile
# Copy both compiled binaries from builder
COPY --from=builder /app/target/release/dp-auth-service /app/dp-auth-service
COPY --from=builder /app/target/release/dp-auth-migrate /app/dp-auth-migrate
```

Update `start.sh`:
```bash
# Run migrations first - exit if this fails
./dp-auth-migrate
```

## Usage Examples

### For Library Consumers

1. **Development usage** (uses all defaults):
```bash
cargo run --bin dp-auth-migrate
```

2. **Production build and run**:
```bash
# Build the binary
cargo build --release --bin dp-auth-migrate

# Run with custom database file
./target/release/dp-auth-migrate --sqlite-file ./production.db
```

3. **Using config file**:
```toml
# dp-auth-migrate.toml
[database]
sqlite_file = "./production.db"

[options]
skip_seeds = true
```

```bash
# Build and run with config
cargo build --release --bin dp-auth-migrate
./target/release/dp-auth-migrate --config ./dp-auth-migrate.toml
```

4. **Environment variables for CI/CD**:
```bash
# Build the binary
cargo build --release --bin dp-auth-migrate

# Set environment and run
export DP_AUTH_MIGRATE_SQLITE_FILE="./test.db"
export DP_AUTH_MIGRATE_SKIP_SEEDS="true"
./target/release/dp-auth-migrate
```

5. **Docker deployment example**:
```dockerfile
# In your Dockerfile
FROM rust:1.88.0 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin my-app --bin dp-auth-migrate

FROM debian:bullseye-slim
WORKDIR /app
COPY --from=builder /app/target/release/my-app /app/my-app
COPY --from=builder /app/target/release/dp-auth-migrate /app/dp-auth-migrate

# Run migrations then start app
CMD ["sh", "-c", "./dp-auth-migrate --sqlite-file ./data/app.db && ./my-app"]
```

## Files to Create/Modify

### New Files
1. `src/migration_config.rs` - Configuration structures and parsing logic
2. `config/scripts/dp_auth_migrate.rs` - New migration binary

### Modified Files
1. `Cargo.toml` - Update binary configuration, add dependencies
2. `src/lib.rs` - Export new modules and Database struct
3. `README.md` - Update documentation
4. `Dockerfile` - Update binary name
5. `start.sh` - Update binary name

### Removed Files
1. `config/scripts/migrate_and_dump.rs` - Replaced by new binary

## Benefits

1. **Better library integration**: Clear binary name and configurable paths
2. **Flexible configuration**: Multiple configuration methods for different use cases
3. **Smart defaults**: Works out of the box for most library consumers
4. **Backward compatibility**: Existing deployments continue to work
5. **Developer experience**: Easy to use with sensible defaults and good documentation
6. **CI/CD friendly**: Environment variable support for automated deployments

## Migration Path

1. **Phase 1**: Implement new binary alongside existing one
2. **Phase 2**: Update documentation and examples
3. **Phase 3**: Update deployment files to use new binary
4. **Phase 4**: Remove old binary in next major version

This approach ensures existing users aren't broken while providing a smooth transition path.