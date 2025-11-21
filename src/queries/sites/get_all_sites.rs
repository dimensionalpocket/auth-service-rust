use crate::models::Site;
use sqlx::SqlitePool;

pub struct GetAllSitesQuery;

impl GetAllSitesQuery {
  pub async fn run(pool: &SqlitePool) -> Result<Vec<Site>, sqlx::Error> {
    sqlx::query_as::<_, Site>(
      "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json 
       FROM sites",
    )
    .fetch_all(pool)
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};

  #[tokio::test]
  async fn test_get_all_sites_empty() {
    let (pool, _tmp) = create_test_database().await;
    let sites = GetAllSitesQuery::run(&pool).await.unwrap();
    assert_eq!(sites.len(), 0);
  }

  #[tokio::test]
  async fn test_get_all_sites_with_data() {
    let (pool, _tmp) = create_test_database().await;

    // Create test sites
    let site1_data = CreateSiteData {
      slug: "site1".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: None,
      metadata_json: None,
    };

    let site2_data = CreateSiteData {
      slug: "site2".to_string(),
      subdomain: None,
      port: Some(80),
      protocol: Some("http".to_string()),
      metadata_json: None,
    };

    CreateSiteQuery::run(&pool, site1_data).await.unwrap();
    CreateSiteQuery::run(&pool, site2_data).await.unwrap();

    let sites = GetAllSitesQuery::run(&pool).await.unwrap();
    assert_eq!(sites.len(), 2);
    assert_eq!(sites[0].slug, "site1");
    assert_eq!(sites[1].slug, "site2");
  }
}
