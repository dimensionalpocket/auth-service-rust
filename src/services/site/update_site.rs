use crate::models::Site;
use crate::queries::sites::{UpdateSiteData, UpdateSiteQuery};
use crate::types::SiteError;
use sqlx::SqliteConnection;

use super::ValidateSiteSlugService;

pub struct UpdateSiteService;

impl UpdateSiteService {
  pub async fn run(
    conn: &mut SqliteConnection,
    id: i64,
    data: UpdateSiteData,
  ) -> Result<Option<Site>, SiteError> {
    // Validate slug if provided
    if let Some(ref slug) = data.slug {
      ValidateSiteSlugService::run(slug)?;

      // Check slug uniqueness (excluding current site)
      let existing_site = sqlx::query_as::<_, Site>(
        "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json FROM sites WHERE slug = ? AND id != ?",
      )
      .bind(slug)
      .bind(id)
      .fetch_optional(&mut *conn)
      .await
      .map_err(SiteError::DatabaseError)?;

      if existing_site.is_some() {
        return Err(SiteError::SlugAlreadyExists(slug.clone()));
      }
    }

    match UpdateSiteQuery::run(conn, UpdateSiteData { id, ..data }).await {
      Ok(Some(site)) => Ok(Some(site)),
      Ok(None) => Err(SiteError::SiteNotFound(id)),
      Err(err) => Err(SiteError::DatabaseError(err)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::site::create_site::CreateSiteService;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_site_success() {
    let mut conn = pool.acquire().await.unwrap();

    let create_data = crate::queries::sites::CreateSiteData {
      slug: "update-test".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: None,
      metadata_json: Some(r#"{"test": true}"#.to_string()),
    };
    let site = CreateSiteService::run(&mut conn, create_data)
      .await
      .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let update_data = UpdateSiteData {
      id: site.id,
      slug: Some("updated-slug".to_string()),
      subdomain: Some(None),
      port: Some(Some(8080)),
      protocol: Some("http".to_string()),
      metadata_json: Some(None),
    };

    let updated_site = UpdateSiteService::run(&mut conn, site.id, update_data)
      .await
      .unwrap()
      .unwrap();

    assert_eq!(updated_site.id, site.id);
    assert_eq!(updated_site.slug, "updated-slug");
    assert_eq!(updated_site.subdomain, None);
    assert_eq!(updated_site.port, Some(8080));
    assert_eq!(updated_site.protocol, "http");
    assert_eq!(updated_site.metadata_json, None);
    assert!(updated_site.updated_ts > site.updated_ts);
    assert_eq!(updated_site.created_ts, site.created_ts);
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_site_slug_already_exists() {
    let mut conn = pool.acquire().await.unwrap();

    let data1 = crate::queries::sites::CreateSiteData {
      slug: "site1".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let data2 = crate::queries::sites::CreateSiteData {
      slug: "site2".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let site1 = CreateSiteService::run(&mut conn, data1).await.unwrap();
    CreateSiteService::run(&mut conn, data2).await.unwrap();

    let update_data = UpdateSiteData {
      id: site1.id,
      slug: Some("site2".to_string()),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = UpdateSiteService::run(&mut conn, site1.id, update_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SlugAlreadyExists(slug) => assert_eq!(slug, "site2"),
      _ => panic!("Expected SlugAlreadyExists error"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_site_not_found() {
    let mut conn = pool.acquire().await.unwrap();

    let update_data = UpdateSiteData {
      id: 999,
      slug: Some("nonexistent".to_string()),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = UpdateSiteService::run(&mut conn, 999, update_data).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SiteNotFound(id) => assert_eq!(id, 999),
      _ => panic!("Expected SiteNotFound error"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_update_site_invalid_slug() {
    let mut conn = pool.acquire().await.unwrap();

    let create_data = crate::queries::sites::CreateSiteData {
      slug: "valid-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let site = CreateSiteService::run(&mut conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateSiteData {
      id: site.id,
      slug: Some("ab".to_string()),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = UpdateSiteService::run(&mut conn, site.id, update_data).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Validation error"));
  }
}
