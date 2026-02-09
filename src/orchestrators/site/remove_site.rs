use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, DeleteSiteService};
use crate::types::SiteError;

pub struct RemoveSiteOrchestrator;

impl RemoveSiteOrchestrator {
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    site_id: i64,
  ) -> Result<crate::models::Site, SiteError> {
    let main_pool = databases.main();
    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(SiteError::DatabaseError)?;

    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(SiteError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut main_conn, user_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::AuthenticationError("User not found".to_string()))?;

    let allowed = CheckUserPermissionService::run(&mut main_conn, &user, "can_delete_site").await?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Delete site
    DeleteSiteService::run(&mut main_conn, site_id).await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
  use crate::test_utils::{create_test_role_with_databases, create_test_user_with_databases};
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_remove_site_with_permission_check_success() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_delete_site"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "site-to-delete".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let site = {
      let mut main_conn = main_pool.acquire().await.unwrap();
      CreateSiteQuery::run(&mut main_conn, create_data)
        .await
        .unwrap()
    };

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test site deletion
    let result = RemoveSiteOrchestrator::run(&databases, session_context, site.id).await;

    assert!(result.is_ok());
    let deleted_site = result.unwrap();
    assert_eq!(deleted_site.id, site.id);
    assert_eq!(deleted_site.slug, "site-to-delete");
  }

  #[dps_auth_db_test]
  async fn test_remove_site_with_permission_check_forbidden() {
    // Create user role without can_delete_site permission
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let regular_user = create_test_user_with_databases(&databases, "user", user_role_id).await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "protected-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let site = {
      let mut main_conn = main_pool.acquire().await.unwrap();
      CreateSiteQuery::run(&mut main_conn, create_data)
        .await
        .unwrap()
    };

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test site deletion
    let result = RemoveSiteOrchestrator::run(&databases, session_context, site.id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }
}
