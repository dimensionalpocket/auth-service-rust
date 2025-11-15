# Remove migration config-file approach — analysis & plan

Date: 2025-11-15

Decision: Immediate removal

Summary

- Goal: remove the TOML config-file flow from migration tooling and rely on CLI args and environment variables only.
- Rationale: reduce complexity, avoid file I/O in CI/containers, and leverage existing env/CLI precedence.

Background

The current implementation reads a TOML config file and merges values with precedence CLI > Config File > Env > Defaults. See:
- [`src/migration_config.rs`](src/migration_config.rs:1)
- [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1)

Analysis

Pros of removing the config-file:
- Simplifies precedence and code paths.
- Removes file I/O and failure modes (missing file, parse errors) from CI and containers.
- Encourages standard 12-factor patterns (env / CLI).

Cons / Risks:
- Breaks local developer workflows that rely on a persistent TOML config.
- Fewer places to encode multi-field defaults if more migration options are introduced later.
- Tests may need updates where they currently create temporary TOML files.

Chosen approach

- Immediate removal (no warnings, no backward compatibility).
- Implications: update code, tests, and documentation in a single change; run full test suite and fix regressions.

Proposed code changes (summary)

- Modify [`src/migration_config.rs`](src/migration_config.rs:1):
  - Remove `config: Option<PathBuf>` from `CliArgs`.
  - Remove `ConfigFile`, `DatabaseConfig`, `OptionsConfig` types and `load_config_file` method.
  - Simplify `from_sources` to merge CLI > Env > Defaults only.
- Modify [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1):
  - Remove CLI flag and help text related to config file.
- Update tests that relied on file-based config to instead use env vars or construct `CliArgs`.
- Update `README.md` and [`config/scripts/migrate.sh`](config/scripts/migrate.sh:1) to document env/CLI usage only.

Code samples (proposed)

```rust
// src/migration_config.rs
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
}

pub struct MigrationConfig {
  pub sqlite_file: PathBuf,
  pub skip_seeds: bool,
}

impl MigrationConfig {
  pub fn from_sources(cli_args: CliArgs) -> Result<Self, Box<dyn std::error::Error>> {
    let sqlite_file = cli_args
      .sqlite_file
      .unwrap_or_else(|| PathBuf::from("data/development.db"));

    let skip_seeds = cli_args.skip_seeds;

    Ok(MigrationConfig { sqlite_file, skip_seeds })
  }
}
```

```rust
// config/scripts/dps_auth_api_migrate.rs
use clap::Parser;
use dps_auth_api::migration_config::{CliArgs, MigrationConfig};
use dps_auth_api::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli_args = CliArgs::parse();
  let config = MigrationConfig::from_sources(cli_args)?;

  println!("🚀 Starting dps-auth-api database migration...");
  println!("   SQLite file: {}", config.sqlite_file.display());
  println!("   Skip seeds: {}", config.skip_seeds);

  let database = Database::new(&config.sqlite_file.display().to_string()).await?;
  database.migrate().await?;

  if !config.skip_seeds {
    database.seed().await?;
  }

  database.dump_schema_to_file("config/database/schema.sql").await?;

  println!("🎉 dps-auth-api database migration completed successfully!");
  Ok(())
}
```

Tests and documentation changes

- Search for `DPS_AUTH_API_MIGRATE_CONFIG_FILE` in the repo and remove usages.
- Update tests that create TOML files; change them to set env vars or construct `CliArgs` directly.
- Update `README.md` and [`config/scripts/migrate.sh`](config/scripts/migrate.sh:1) to remove references to a TOML config.
- Ensure CI uses env/CLI-based configuration for integration tests.

Implementation steps

1. Run project-wide search for `DPS_AUTH_API_MIGRATE_CONFIG_FILE` and config-file usage.
2. Edit [`src/migration_config.rs`](src/migration_config.rs:1) to remove file-loading code and related types (apply patch).
3. Edit [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1) to match new API (apply patch).
4. Update tests in `tests/` to remove file-based config dependency.
5. Update `config/scripts/migrate.sh` and `README.md` to reflect immediate removal.
6. Run tests: `mise exec -- cargo test` and fix regressions.

Rollback & mitigation

- Restore previous `src/migration_config.rs` from VCS if necessary.
- Reintroduce a loader function if an urgent use-case is discovered.

Files to be modified

- [`src/migration_config.rs`](src/migration_config.rs:1)
- [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1)
- Tests under [`tests/`](tests:1)
- `README.md` and `config/scripts/migrate.sh`

Approval and next steps

- This plan implements immediate removal. If you approve, I will switch to code mode and apply the changes (update todos as I progress).