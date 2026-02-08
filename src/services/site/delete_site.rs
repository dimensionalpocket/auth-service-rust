use crate::models::Site;
use crate::queries::sites::DeleteSiteQuery;
use crate::types::SiteError;
use sqlx::SqliteConnection;

pub struct DeleteSiteService;

impl DeleteSiteService {
  pub async fn run(conn: &mut SqliteConnection, site_id: i64) -> Result<Site, SiteError> {
    DeleteSiteQuery::run(conn, site_id)
      .await
      .map_err(|e| match e {
        sqlx::Error::RowNotFound => SiteError::SiteNotFound(site_id),
        _ => SiteError::DatabaseError(e),
      })
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::services::site::create_site::CreateSiteService;
  use crate::services::site::get_all_sites::GetAllSitesService;

  #[dps_auth_db_test]
  async fn test_delete_site_success() {
    let mut conn = main_pool.acquire().await.unwrap();

    let create_data = crate::queries::sites::CreateSiteData {
      slug: "delete-test".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some(r#"{"test": true}"#.to_string()),
    };
    let site = CreateSiteService::run(&mut conn, create_data)
      .await
      .unwrap();

    let deleted_site = DeleteSiteService::run(&mut conn, site.id).await.unwrap();

    assert_eq!(deleted_site.id, site.id);
    assert_eq!(deleted_site.slug, site.slug);
    assert_eq!(deleted_site.subdomain, site.subdomain);
    assert_eq!(deleted_site.port, site.port);
    assert_eq!(deleted_site.protocol, site.protocol);
    assert_eq!(deleted_site.metadata_json, site.metadata_json);
    assert_eq!(deleted_site.created_ts, site.created_ts);
    assert_eq!(deleted_site.updated_ts, site.updated_ts);

    let all_sites = GetAllSitesService::run(&mut conn).await.unwrap();
    assert_eq!(all_sites.len(), 0);
  }

  #[dps_auth_db_test]
  async fn test_delete_site_not_found() {
    let mut conn = main_pool.acquire().await.unwrap();

    let result = DeleteSiteService::run(&mut conn, 999).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SiteNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected SiteNotFound error"),
    }
  }
}
