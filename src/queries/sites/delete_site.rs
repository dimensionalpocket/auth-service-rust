use crate::models::Site;
use sqlx::SqliteConnection;

pub struct DeleteSiteQuery;

impl DeleteSiteQuery {
  pub async fn run(conn: &mut SqliteConnection, site_id: i64) -> Result<Site, sqlx::Error> {
    // First fetch the site to return its data
    let site = sqlx::query_as::<_, Site>(
      "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json FROM sites WHERE id = ?"
    )
    .bind(site_id)
    .fetch_one(&mut *conn)
    .await?;

    // Then delete the site
    let result = sqlx::query("DELETE FROM sites WHERE id = ?")
      .bind(site_id)
      .execute(&mut *conn)
      .await?;

    if result.rows_affected() == 0 {
      return Err(sqlx::Error::RowNotFound);
    }

    Ok(site)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
  use crate::test_utils::create_test_database;
  use sqlx::Row;

  #[tokio::test]
  async fn test_delete_site_success() {
    let (pool, _tmp) = create_test_database().await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let site = CreateSiteQuery::run(&pool, create_data).await.unwrap();

    // Delete the site and get returned data
    let mut conn = pool.acquire().await.unwrap();
    let deleted_site = DeleteSiteQuery::run(&mut conn, site.id).await.unwrap();

    // Verify returned data matches original
    assert_eq!(deleted_site.id, site.id);
    assert_eq!(deleted_site.slug, site.slug);
    assert_eq!(deleted_site.subdomain, site.subdomain);
    assert_eq!(deleted_site.port, site.port);
    assert_eq!(deleted_site.protocol, site.protocol);
    assert_eq!(deleted_site.metadata_json, site.metadata_json);
    assert_eq!(deleted_site.created_ts, site.created_ts);
    assert_eq!(deleted_site.updated_ts, site.updated_ts);

    // Verify site is deleted from database
    let result = sqlx::query("SELECT COUNT(*) FROM sites WHERE id = ?")
      .bind(site.id)
      .fetch_one(&mut *conn)
      .await
      .unwrap();
    let count: i64 = result.get(0);
    assert_eq!(count, 0);
  }

  #[tokio::test]
  async fn test_delete_site_not_found() {
    let (pool, _tmp) = create_test_database().await;
    let mut conn = pool.acquire().await.unwrap();

    // Try to delete non-existent site
    let result = DeleteSiteQuery::run(&mut conn, 999).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), sqlx::Error::RowNotFound));
  }
}
