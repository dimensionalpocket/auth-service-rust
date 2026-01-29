use crate::middleware::session::SessionContext;
use crate::queries::sites::UpdateSiteData;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, UpdateSiteService};
use crate::types::SiteError;
use sqlx::SqlitePool;

pub struct UpdateSiteOrchestrator;

impl UpdateSiteOrchestrator {
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
    site_id: i64,
    update_data: UpdateSiteData,
  ) -> Result<Option<crate::models::Site>, SiteError> {
    let mut conn = pool.acquire().await.map_err(SiteError::DatabaseError)?;

    // Authentication: Check if user is authenticated
    let user_id = session_context
      .user_id()
      .ok_or(SiteError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut conn, user_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::AuthenticationError("User not found".to_string()))?;

    let allowed = CheckUserPermissionService::run(&mut conn, &user, "can_update_site").await?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Update site
    UpdateSiteService::run(&mut conn, site_id, update_data).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery, UpdateSiteData};
  use crate::test_utils::{
    create_test_database, create_test_role_with_pool, create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[tokio::test]
  async fn test_update_site_with_permission_check_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_update_site"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "original-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let site = {
      let mut conn = pool.acquire().await.unwrap();
      CreateSiteQuery::run(&mut conn, create_data).await.unwrap()
    };

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test site update
    let update_data = UpdateSiteData {
      id: site.id,
      slug: Some("updated-site".to_string()),
      subdomain: Some(Some("updated".to_string())),
      port: Some(Some(8080)),
      protocol: Some("http".to_string()),
      metadata_json: Some(Some("{\"updated\": true}".to_string())),
    };

    let result = UpdateSiteOrchestrator::run(&pool, session_context, site.id, update_data).await;

    assert!(result.is_ok());
    let updated_site = result.unwrap();
    assert!(updated_site.is_some());
    let site = updated_site.unwrap();
    assert_eq!(site.slug, "updated-site");
    assert_eq!(site.subdomain, Some("updated".to_string()));
    assert_eq!(site.port, Some(8080));
    assert_eq!(site.protocol, "http");
  }

  #[tokio::test]
  async fn test_update_site_with_permission_check_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without can_update_site permission
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let regular_user = create_test_user_with_pool(&pool, "user", user_role_id).await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "protected-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let site = {
      let mut conn = pool.acquire().await.unwrap();
      CreateSiteQuery::run(&mut conn, create_data).await.unwrap()
    };

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test site update
    let update_data = UpdateSiteData {
      id: site.id,
      slug: Some("forbidden-update".to_string()),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = UpdateSiteOrchestrator::run(&pool, session_context, site.id, update_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }
}
