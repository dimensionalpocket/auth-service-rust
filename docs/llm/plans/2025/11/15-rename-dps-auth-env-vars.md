# Plan: synchronize DPS_AUTH_ → DPS_AUTH_API_ env var prefixes

Summary

Goal: align migration env variable names with the rest of the project by changing only the prefix from DPS_AUTH_ to DPS_AUTH_API_ for migration-related environment variables.

Scope

- Code: migration binary and helper scripts.
- README examples and shell scripts.
- Tests and CI.
- Project root [.env](.env:1) file (gitignored).

Rename rule

- For every migration-related environment variable that starts with DPS_AUTH_, change it to DPS_AUTH_API_<suffix>. Example: DPS_AUTH_SQLITE_FILE → DPS_AUTH_API_SQLITE_FILE. Only the prefix changes.

Known occurrences to update

- [`src/migration_config.rs`](src/migration_config.rs:11) — DPS_AUTH_SQLITE_FILE → DPS_AUTH_API_SQLITE_FILE
- [`src/migration_config.rs`](src/migration_config.rs:15) — DPS_AUTH_MIGRATE_CONFIG_FILE → DPS_AUTH_API_MIGRATE_CONFIG_FILE
- [`src/migration_config.rs`](src/migration_config.rs:19) — DPS_AUTH_MIGRATE_SKIP_SEEDS → DPS_AUTH_API_MIGRATE_SKIP_SEEDS
- [`README.md`](README.md:212) — export DPS_AUTH_SQLITE_FILE → export DPS_AUTH_API_SQLITE_FILE
- [`README.md`](README.md:248) — export DPS_AUTH_SQLITE_FILE → export DPS_AUTH_API_SQLITE_FILE
- [`tests/integration_tests.rs`](tests/integration_tests.rs:242) — DPS_AUTH_INSECURE_COOKIE → DPS_AUTH_API_INSECURE_COOKIE
- [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1) — migration script env lookups
- [`config/scripts/migrate.sh`](config/scripts/migrate.sh:1) — shell script exports and usage
- [`.github/workflows/docker-build-test.yml`](.github/workflows/docker-build-test.yml:63) — verify env usage
- [`.env`](.env:1) — project root dotenv file (update variable names)

Note: a repo scan returned 26 matches for DPS_AUTH_*; the list above covers the key locations found during the initial scan. Do not perform any additional repository-wide searches during implementation; use the inventory gathered during planning.

Implementation plan (high level)

1. Inventory: use the repository inventory produced during planning; do not run new scans.
2. Update migration CLI args/attributes: change environment variable names in [`src/migration_config.rs`](src/migration_config.rs:11) to use DPS_AUTH_API_*.
3. Update helper scripts and shell scripts: [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1) and [`config/scripts/migrate.sh`](config/scripts/migrate.sh:1).
4. Update README examples: replace example exports in [`README.md`](README.md:212) and related README sections.
5. Update tests and CI: replace occurrences in tests (e.g., [`tests/integration_tests.rs`](tests/integration_tests.rs:242)) and update CI workflows as needed.
6. Update project root dotenv: edit [`.env`](.env:1) to use DPS_AUTH_API_* variable names.
7. Run tests locally and iterate until green (use mise exec -- cargo test).
8. Prepare changes for review; do not perform git operations.

Code sample — change in `src/migration_config.rs`

```rust
#[derive(Debug, Parser)]
pub struct MigrationConfig {
    /// Path to SQLite database file
    #[arg(long, env = "DPS_AUTH_API_SQLITE_FILE")]
    pub sqlite_file: Option<PathBuf>,

    /// Path to configuration file
    #[arg(long, env = "DPS_AUTH_API_MIGRATE_CONFIG_FILE")]
    pub config: Option<PathBuf>,

    /// Skip running seed files
    #[arg(long, env = "DPS_AUTH_API_MIGRATE_SKIP_SEEDS")]
    pub skip_seeds: bool,
}
```

Testing notes

- Run unit and integration tests with: mise exec -- cargo test
- Validate migration binary behavior manually by setting the new DPS_AUTH_API_* variables and confirming migrations run.

Risks & mitigations

- Missing occurrences: mitigate by a strict pre-change inventory and repo-wide replacement pass.
- Breaking automation/scripts: coordinate updates to scripts, README and .env simultaneously to avoid breakage.

Files to modify (summary)

- [`src/migration_config.rs`](src/migration_config.rs:11)
- [`config/scripts/dps_auth_api_migrate.rs`](config/scripts/dps_auth_api_migrate.rs:1)
- [`config/scripts/migrate.sh`](config/scripts/migrate.sh:1)
- [`README.md`](README.md:212)
- [`tests/integration_tests.rs`](tests/integration_tests.rs:242)
- [`.github/workflows/docker-build-test.yml`](.github/workflows/docker-build-test.yml:63)
- [`.env`](.env:1)

Next actions (after plan approval)

- I will switch to code mode and implement the renames in the order above when you confirm.

End