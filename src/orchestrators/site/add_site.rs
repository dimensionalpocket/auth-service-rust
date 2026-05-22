use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::queries::sites::CreateSiteData;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, CreateSiteService};
use crate::types::SiteError;
use crate::utils::session_context_sub_to_user_id;
use dps_config::DpsConfig;

pub struct AddSiteOrchestrator;

impl AddSiteOrchestrator {
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    config: &DpsConfig,
    create_data: CreateSiteData,
  ) -> Result<crate::models::Site, SiteError> {
    let main_pool = databases.main();
    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(SiteError::DatabaseError)?;

    // Authentication: Check if user is authenticated
    let user_id = session_context_sub_to_user_id(&session_context, config)
      .map_err(SiteError::AuthenticationError)?;

    // Authorization: Get user and check permissions
    let user = GetUserByIdQuery::run(&mut main_conn, user_id)
      .await
      .map_err(SiteError::DatabaseError)?
      .ok_or(SiteError::AuthenticationError("User not found".to_string()))?;

    let allowed = CheckUserPermissionService::run(&mut main_conn, &user, "can_create_site").await?;

    if !allowed {
      return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }

    // Business logic: Create site
    CreateSiteService::run(&mut main_conn, create_data).await
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;

  use crate::test_utils::{
    create_test_dps_config, create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_create_site_with_permission_check_success() {
    // Create admin role and user
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_create_site"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    // Create session context for admin user
    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
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

    let result = AddSiteOrchestrator::run(
      &databases,
      session_context,
      &create_test_dps_config(),
      create_data,
    )
    .await;

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

    let result = AddSiteOrchestrator::run(
      &databases,
      session_context,
      &create_test_dps_config(),
      create_data,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::AuthenticationError(msg) => {
        assert!(msg.contains("No valid session"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_create_site_with_permission_check_nonexistent_user() {
    // Create session context for non-existent user
    let session_payload = DpsAuthSessionPayload {
      sub: "999".to_string(),
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

    let result = AddSiteOrchestrator::run(
      &databases,
      session_context,
      &create_test_dps_config(),
      create_data,
    )
    .await;

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
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let regular_user = create_test_user_with_databases(&databases, "user", user_role_id).await;

    // Create session context for regular user
    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id.to_string(),
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

    let result = AddSiteOrchestrator::run(
      &databases,
      session_context,
      &create_test_dps_config(),
      create_data,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SiteError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }
}
