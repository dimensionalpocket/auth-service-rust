# Create Site model and queries

Goal

- Add Site model mirroring config/database/migrations/003_create_sites.sql.
- Add queries service src/queries/sites/ with create_site and tests.

Files to create/modify

- [`src/models/site.rs`](src/models/site.rs:1)
- [`src/models/mod.rs`](src/models/mod.rs:1) (export)
- [`src/queries/sites/create_site.rs`](src/queries/sites/create_site.rs:1)
- [`src/queries/sites/mod.rs`](src/queries/sites/mod.rs:1)
- [`src/queries/mod.rs`](src/queries/mod.rs:1) (export)
- [`docs/llm/plans/2025/11/20-create-site-model.md`](docs/llm/plans/2025/11/20-create-site-model.md:1) (this plan)

Background / References

- Migration: [`config/database/migrations/003_create_sites.sql`](config/database/migrations/003_create_sites.sql:1)
- Example model: [`src/models/user.rs`](src/models/user.rs:1)
- Example query: [`src/queries/users/create_user.rs`](src/queries/users/create_user.rs:1)

Implementation details

Model: create [`src/models/site.rs`](src/models/site.rs:1)

Code sample:

```rust
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct Site {
  pub id: i64,
  pub created_ts: i64,
  pub updated_ts: i64,
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: String,
  pub metadata_json: Option<String>,
}
```

Export model in [`src/models/mod.rs`](src/models/mod.rs:1):

```rust
pub mod site;
pub use site::Site;
```

Queries: directory [`src/queries/sites/`](src/queries/sites/create_site.rs:1)

Create [`src/queries/sites/create_site.rs`](src/queries/sites/create_site.rs:1) with pattern from create_user.rs.

Code sample:

```rust
use crate::models::Site;
use sqlx::SqlitePool;

#[derive(Debug)]
pub struct CreateSiteData {
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: Option<String>, // default to "https" if None
  pub metadata_json: Option<String>,
}

pub struct CreateSiteQuery;

impl CreateSiteQuery {
  pub async fn run(pool: &SqlitePool, data: CreateSiteData) -> Result<Site, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    let protocol = data.protocol.unwrap_or_else(|| "https".to_string());

    let result = sqlx::query(
      r#"
      INSERT INTO sites (created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json)
      VALUES (?, ?, ?, ?, ?, ?, ?)
      "#,
    )
    .bind(now)
    .bind(now)
    .bind(&data.slug)
    .bind(&data.subdomain)
    .bind(&data.port)
    .bind(&protocol)
    .bind(&data.metadata_json)
    .execute(pool)
    .await?;

    let site_id = result.last_insert_rowid();

    sqlx::query_as::<_, Site>(
      "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json FROM sites WHERE id = ?"
    )
    .bind(site_id)
    .fetch_one(pool)
    .await
  }
}
```

Export queries: add [`src/queries/sites/mod.rs`](src/queries/sites/mod.rs:1) with:

```rust
pub mod create_site;
pub use create_site::{CreateSiteData, CreateSiteQuery};
```

And update [`src/queries/mod.rs`](src/queries/mod.rs:1) to `pub mod sites;`

Tests

- Add tests inside `#[cfg(test)] mod tests` in `create_site.rs`, following patterns from [`src/queries/users/create_user.rs`](src/queries/users/create_user.rs:1).
- Tests to include:
  - test_create_site_success: create a site and assert fields, protocol defaults to "https".
  - test_create_site_duplicate_slug_fails: inserting same slug twice errors.
  - test_create_site_nullable_fields: subdomain/port/metadata can be NULL.

Example test snippet:

```rust
#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;

  #[tokio::test]
  async fn test_create_site_success() {
    let (pool, _tmp) = create_test_database().await;

    let data = CreateSiteData {
      slug: "example".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: None,
      metadata_json: Some(r#"{"a":1}"#.to_string()),
    };

    let site = CreateSiteQuery::run(&pool, data).await.unwrap();
    assert_eq!(site.slug, "example");
    assert_eq!(site.protocol, "https");
    assert_eq!(site.subdomain, Some("www".to_string()));
  }
}
```

Test run

- Run tests with: mise exec -- cargo test
- If failures occur, iterate and fix.

Risks and notes

- Ensure `sqlx::FromRow` import is present and the selected columns match struct order.
- The migration enforces UNIQUE(slug); tests should assert duplicate inserts fail.
- Follow docs/llm/instructions.md: do not implement until plan approved.

Next steps

- If you approve this plan I will implement the changes described and run the tests.

----