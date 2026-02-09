use crate::database::Databases;
use crate::models::Site;
use crate::services::GetAllSitesService;
use crate::types::SiteError;

pub struct GetSitesOrchestrator;

impl GetSitesOrchestrator {
  pub async fn run(databases: &Databases) -> Result<Vec<Site>, SiteError> {
    let main_pool = databases.main();
    let mut conn = main_pool
      .acquire()
      .await
      .map_err(SiteError::DatabaseError)?;

    GetAllSitesService::run(&mut conn).await
  }
}
