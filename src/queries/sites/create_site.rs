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
    .bind(data.port)
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::create_test_database;

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
    assert_eq!(site.port, Some(443));
    assert_eq!(site.metadata_json, Some(r#"{"a":1}"#.to_string()));
    assert!(site.created_ts > 0);
    assert_eq!(site.created_ts, site.updated_ts);
  }

  #[tokio::test]
  async fn test_create_site_duplicate_slug_fails() {
    let (pool, _tmp) = create_test_database().await;

    let data1 = CreateSiteData {
      slug: "duplicate".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    CreateSiteQuery::run(&pool, data1).await.unwrap();

    let data2 = CreateSiteData {
      slug: "duplicate".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(80),
      protocol: Some("http".to_string()),
      metadata_json: None,
    };

    let result = CreateSiteQuery::run(&pool, data2).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_create_site_nullable_fields() {
    let (pool, _tmp) = create_test_database().await;

    let data = CreateSiteData {
      slug: "nullable".to_string(),
      subdomain: None,
      port: None,
      protocol: Some("http".to_string()),
      metadata_json: None,
    };

    let site = CreateSiteQuery::run(&pool, data).await.unwrap();

    assert_eq!(site.slug, "nullable");
    assert_eq!(site.protocol, "http");
    assert_eq!(site.subdomain, None);
    assert_eq!(site.port, None);
    assert_eq!(site.metadata_json, None);
  }
}
