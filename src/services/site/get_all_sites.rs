use crate::models::Site;
use crate::queries::sites::GetAllSitesQuery;
use crate::types::SiteError;
use sqlx::SqliteConnection;

pub struct GetAllSitesService;

impl GetAllSitesService {
  pub async fn run(conn: &mut SqliteConnection) -> Result<Vec<Site>, SiteError> {
    GetAllSitesQuery::run(conn)
      .await
      .map_err(SiteError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::site::create_site::CreateSiteService;
  use crate::test_utils::create_test_database;

  #[tokio::test]
  async fn test_get_all_sites() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();
    let mut conn = pool.acquire().await.unwrap();

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

    CreateSiteService::run(&mut conn, data1).await.unwrap();
    CreateSiteService::run(&mut conn, data2).await.unwrap();

    let sites = GetAllSitesService::run(&mut conn).await.unwrap();
    assert_eq!(sites.len(), 2);
    assert_eq!(sites[0].slug, "test1");
    assert_eq!(sites[1].slug, "test2");
  }
}
