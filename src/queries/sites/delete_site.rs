use sqlx::SqlitePool;

pub struct DeleteSiteQuery;

impl DeleteSiteQuery {
  pub async fn run(pool: &SqlitePool, site_id: i64) -> Result<(), sqlx::Error> {
    let result = sqlx::query("DELETE FROM sites WHERE id = ?")
      .bind(site_id)
      .execute(pool)
      .await?;

    if result.rows_affected() == 0 {
      return Err(sqlx::Error::RowNotFound);
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
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

    // Delete the site
    DeleteSiteQuery::run(&pool, site.id).await.unwrap();

    // Verify site is deleted
    let result = sqlx::query("SELECT COUNT(*) FROM sites WHERE id = ?")
      .bind(site.id)
      .fetch_one(&pool)
      .await
      .unwrap();
    let count: i64 = result.get(0);
    assert_eq!(count, 0);
  }

  #[tokio::test]
  async fn test_delete_site_not_found() {
    let (pool, _tmp) = create_test_database().await;

    // Try to delete non-existent site
    let result = DeleteSiteQuery::run(&pool, 999).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), sqlx::Error::RowNotFound));
  }
}
