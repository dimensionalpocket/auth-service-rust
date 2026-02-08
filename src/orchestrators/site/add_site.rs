use crate::middleware::session::SessionContext;
use crate::queries::sites::CreateSiteData;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, CreateSiteService};
use crate::types::SiteError;
use sqlx::SqlitePool;

pub struct AddSiteOrchestrator;

impl AddSiteOrchestrator {
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
    create_data: CreateSiteData,
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

    let allowed = CheckUserPermissionService::run(&mut conn, &user, "can_create_site").await?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Create site
    CreateSiteService::run(&mut conn, create_data).await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;

  use crate::test_utils::{create_test_role_with_pool, create_test_user_with_pool};
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_create_site_with_permission_check_success() {
    // Create admin role and user
    let admin_role_id = create_test_role_with_pool(&main_pool, "admin", &["can_create_site"]).await;
    let admin_user = create_test_user_with_pool(&main_pool, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    // Test site creation
    let create_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some("{\"description\": \"Test site\"}".to_string()),
    };

    let result = AddSiteOrchestrator::run(&main_pool, session_context, create_data).await;

    assert!(result.is_ok());
    let site = result.unwrap();
    assert_eq!(site.slug, "test-site");
    assert_eq!(site.subdomain, Some("www".to_string()));
    assert_eq!(site.port, Some(443));
    assert_eq!(site.protocol, "https");
  }

  #[dps_auth_db_test]
  async fn test_create_site_with_permission_check_unauthenticated() {
    // Create session context without user (not authenticated)
    let session_context = SessionContext::new(None);

    let create_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = AddSiteOrchestrator::run(&main_pool, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_site_with_permission_check_nonexistent_user() {
    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let create_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = AddSiteOrchestrator::run(&main_pool, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::AuthenticationError(msg) => {
        assert!(msg.contains("User not found"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_site_with_permission_check_forbidden() {
    // Create user role without can_create_site permission
    let user_role_id = create_test_role_with_pool(&main_pool, "user", &[]).await;
    let regular_user = create_test_user_with_pool(&main_pool, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let create_data = CreateSiteData {
      slug: "forbidden-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };

    let result = AddSiteOrchestrator::run(&main_pool, session_context, create_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }
}
