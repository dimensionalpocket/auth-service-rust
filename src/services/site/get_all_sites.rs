use crate::models::Site;
use crate::queries::sites::GetAllSitesQuery;
use crate::types::SiteError;
use sqlx::SqliteConnection;

pub struct GetAllSitesService;

impl GetAllSitesService {
  pub async fn run(main_conn: &mut SqliteConnection) -> Result<Vec<Site>, SiteError> {
    GetAllSitesQuery::run(main_conn)
      .await
      .map_err(SiteError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::services::site::create_site::CreateSiteService;

  #[dps_auth_db_test]
  async fn test_get_all_sites() {
    let mut main_conn = main_pool.acquire().await.unwrap();

    let data1 = crate::queries::sites::CreateSiteData {
      slug: "test1".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: None,
      metadata_json: None,
    };

    let data2 = crate::queries::sites::CreateSiteData {
      slug: "test2".to_string(),
      subdomain: None,
      port: Some(80),
      protocol: Some("http".to_string()),
      metadata_json: None,
    };

    CreateSiteService::run(&mut main_conn, data1).await.unwrap();
    CreateSiteService::run(&mut main_conn, data2).await.unwrap();

    let sites = GetAllSitesService::run(&mut main_conn).await.unwrap();
    assert_eq!(sites.len(), 2);
    assert_eq!(sites[0].slug, "test1");
    assert_eq!(sites[1].slug, "test2");
  }
}
