use crate::middleware::session::SessionContext;
use crate::queries::sites::GetSiteByIdQuery;
use crate::queries::users::GetUserByIdQuery;
use crate::services::RoleService;
use crate::types::SiteError;
use sqlx::SqlitePool;

pub struct GetSiteOrchestrator;

impl GetSiteOrchestrator {
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
    site_id: i64,
  ) -> Result<crate::models::Site, SiteError> {
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

    let allowed =
      RoleService::check_user_permission(&mut conn, &user, "can_view_site_details").await?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Get site details
    GetSiteByIdQuery::run(&mut conn, site_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::SiteNotFound(site_id))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
  use crate::test_utils::{
    create_test_database, create_test_role_with_pool, create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[tokio::test]
  async fn test_get_site_details_with_permission_check_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["can_view_site_details"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "detailed-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some("{\"description\": \"Test site\"}".to_string()),
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

    // Test site details retrieval
    let result = GetSiteOrchestrator::run(&pool, session_context, site.id).await;

    assert!(result.is_ok());
    let retrieved_site = result.unwrap();
    assert_eq!(retrieved_site.id, site.id);
    assert_eq!(retrieved_site.slug, "detailed-site");
    assert_eq!(retrieved_site.subdomain, Some("www".to_string()));
    assert_eq!(retrieved_site.port, Some(443));
    assert_eq!(retrieved_site.protocol, "https");
  }

  #[tokio::test]
  async fn test_get_site_details_with_permission_check_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without can_view_site_details permission
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

    // Test site details retrieval
    let result = GetSiteOrchestrator::run(&pool, session_context, site.id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[tokio::test]
  async fn test_get_site_details_with_permission_check_site_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role and user
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["can_view_site_details"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test site details retrieval for non-existent site
    let result = GetSiteOrchestrator::run(&pool, session_context, 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::SiteNotFound(site_id) => {
        assert_eq!(site_id, 999);
      }
      _ => panic!("Expected SiteNotFound"),
    }
  }
}
