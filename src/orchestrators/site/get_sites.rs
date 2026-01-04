use crate::models::Site;
use crate::services::SiteService;
use crate::types::SiteError;
use sqlx::SqlitePool;

pub struct GetSitesOrchestrator;

impl GetSitesOrchestrator {
  pub async fn run(pool: &SqlitePool) -> Result<Vec<Site>, SiteError> {
    let mut conn = pool.acquire().await.map_err(SiteError::DatabaseError)?;

    SiteService::get_all_sites(&mut conn).await
  }
}
