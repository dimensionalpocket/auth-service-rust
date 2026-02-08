use crate::models::Site;
use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
use crate::types::SiteError;
use sqlx::SqliteConnection;

use super::ValidateSiteSlugService;

pub struct CreateSiteService;

impl CreateSiteService {
  pub async fn run(conn: &mut SqliteConnection, data: CreateSiteData) -> Result<Site, SiteError> {
    ValidateSiteSlugService::run(&data.slug)?;

    // Clone slug for error handling before moving data
    let slug_clone = data.slug.clone();

    CreateSiteQuery::run(conn, data).await.map_err(|err| {
      if let Some(sqlite_err) = err.as_database_error() {
        if let Some(code) = sqlite_err.code() {
          if code == "1555" || code == "2067" {
            return SiteError::SlugAlreadyExists(slug_clone);
          }
        }
      }
      SiteError::DatabaseError(err)
    })
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  #[dps_auth_db_test]
  async fn test_create_site_success() {
    let mut conn = main_pool.acquire().await.unwrap();

    let data = CreateSiteData {
      slug: "example".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: None,
      metadata_json: Some(r#"{"a":1}"#.to_string()),
    };

    let site = CreateSiteService::run(&mut conn, data).await.unwrap();

    assert_eq!(site.slug, "example");
    assert_eq!(site.protocol, "https");
    assert_eq!(site.subdomain, Some("www".to_string()));
    assert_eq!(site.port, Some(443));
    assert_eq!(site.metadata_json, Some(r#"{"a":1}"#.to_string()));
    assert!(site.created_ts > 0);
    assert_eq!(site.created_ts, site.updated_ts);
  }

  #[dps_auth_db_test]
  async fn test_create_site_slug_already_exists() {
    let mut conn = main_pool.acquire().await.unwrap();

    let data1 = CreateSiteData {
      slug: "duplicate".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    CreateSiteService::run(&mut conn, data1).await.unwrap();

    let data2 = CreateSiteData {
      slug: "duplicate".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(80),
      protocol: Some("http".to_string()),
      metadata_json: None,
    };

    let result = CreateSiteService::run(&mut conn, data2).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SlugAlreadyExists(slug) => assert_eq!(slug, "duplicate"),
      _ => panic!("Expected SlugAlreadyExists error"),
    }
  }
}
