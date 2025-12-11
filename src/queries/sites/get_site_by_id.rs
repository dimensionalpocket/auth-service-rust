use crate::models::Site;
use sqlx::SqlitePool;

pub struct GetSiteByIdQuery;

impl GetSiteByIdQuery {
  pub async fn run(pool: &SqlitePool, site_id: i64) -> Result<Option<Site>, sqlx::Error> {
    let site = sqlx::query_as::<_, Site>(
      "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json 
             FROM sites WHERE id = ?",
    )
    .bind(site_id)
    .fetch_optional(pool)
    .await?;

    Ok(site)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
  use crate::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_site_by_id_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a test site first
    let site_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some("{\"description\": \"Test site\"}".to_string()),
    };
    let created_site = CreateSiteQuery::run(&pool, site_data).await.unwrap();

    // Test getting the site by ID
    let retrieved_site = GetSiteByIdQuery::run(&pool, created_site.id).await.unwrap();

    assert!(retrieved_site.is_some());
    let site = retrieved_site.unwrap();

    assert_eq!(site.id, created_site.id);
    assert_eq!(site.slug, "test-site");
    assert_eq!(site.subdomain, Some("www".to_string()));
    assert_eq!(site.port, Some(443));
    assert_eq!(site.protocol, "https");
    assert_eq!(
      site.metadata_json,
      Some("{\"description\": \"Test site\"}".to_string())
    );
    assert!(site.created_ts > 0);
    assert!(site.updated_ts > 0);
  }

  #[tokio::test]
  async fn test_get_site_by_id_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Test getting a non-existent site
    let retrieved_site = GetSiteByIdQuery::run(&pool, 999).await.unwrap();

    assert!(retrieved_site.is_none());
  }

  #[tokio::test]
  async fn test_get_site_by_id_empty_database() {
    let (pool, _temp_file) = create_test_database().await;

    // Test getting a site from empty database
    let retrieved_site = GetSiteByIdQuery::run(&pool, 1).await.unwrap();

    assert!(retrieved_site.is_none());
  }

  #[tokio::test]
  async fn test_get_site_by_id_multiple_sites() {
    let (pool, _temp_file) = create_test_database().await;

    // Create multiple test sites
    let site1_data = CreateSiteData {
      slug: "site-one".to_string(),
      subdomain: None,
      port: Some(80),
      protocol: Some("http".to_string()),
      metadata_json: None,
    };
    let site2_data = CreateSiteData {
      slug: "site-two".to_string(),
      subdomain: Some("api".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some("{\"type\": \"api\"}".to_string()),
    };

    let created_site1 = CreateSiteQuery::run(&pool, site1_data).await.unwrap();
    let created_site2 = CreateSiteQuery::run(&pool, site2_data).await.unwrap();

    // Test getting the first site
    let retrieved_site1 = GetSiteByIdQuery::run(&pool, created_site1.id)
      .await
      .unwrap();
    assert!(retrieved_site1.is_some());
    let site1 = retrieved_site1.unwrap();
    assert_eq!(site1.slug, "site-one");
    assert_eq!(site1.port, Some(80));
    assert_eq!(site1.protocol, "http");
    assert_eq!(site1.metadata_json, None);

    // Test getting the second site
    let retrieved_site2 = GetSiteByIdQuery::run(&pool, created_site2.id)
      .await
      .unwrap();
    assert!(retrieved_site2.is_some());
    let site2 = retrieved_site2.unwrap();
    assert_eq!(site2.slug, "site-two");
    assert_eq!(site2.subdomain, Some("api".to_string()));
    assert_eq!(site2.port, Some(443));
    assert_eq!(site2.protocol, "https");
    assert_eq!(site2.metadata_json, Some("{\"type\": \"api\"}".to_string()));
  }

  #[tokio::test]
  async fn test_get_site_by_id_with_null_fields() {
    let (pool, _temp_file) = create_test_database().await;

    // Create a site with null optional fields
    let site_data = CreateSiteData {
      slug: "minimal-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None, // Will default to "https"
      metadata_json: None,
    };
    let created_site = CreateSiteQuery::run(&pool, site_data).await.unwrap();

    // Test getting the site
    let retrieved_site = GetSiteByIdQuery::run(&pool, created_site.id).await.unwrap();

    assert!(retrieved_site.is_some());
    let site = retrieved_site.unwrap();

    assert_eq!(site.slug, "minimal-site");
    assert_eq!(site.subdomain, None);
    assert_eq!(site.port, None);
    assert_eq!(site.protocol, "https"); // Default value
    assert_eq!(site.metadata_json, None);
  }
}
