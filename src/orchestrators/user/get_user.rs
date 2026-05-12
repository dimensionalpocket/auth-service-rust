use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::models::user::UserWithRole;
use crate::queries::users::{GetUserByIdQuery, GetUserByIdWithRoleQuery};
use crate::services::CheckUserPermissionService;
use crate::types::{DpsAuthApiConfig, UserError};
use crate::utils::session_context_sub_to_user_id;

pub struct GetUserOrchestrator;

impl GetUserOrchestrator {
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    config: &DpsAuthApiConfig,
    target_user_id: i64,
  ) -> Result<UserWithRole, UserError> {
    let user_id = session_context_sub_to_user_id(&session_context, config)
      .map_err(UserError::AuthenticationError)?;

    let main_pool = databases.main();
    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(UserError::DatabaseError)?;

    let user = GetUserByIdQuery::run(&mut main_conn, user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(user_id))?;

    let allowed =
      CheckUserPermissionService::run(&mut main_conn, &user, "can_view_user_details").await?;

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    GetUserByIdWithRoleQuery::run(&mut main_conn, target_user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(target_user_id))
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_config, create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_get_user_details_with_permission_check_success() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_view_user_details"]).await;
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;

    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;
    let target_user =
      create_test_user_with_databases(&databases, "target_user", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      target_user.id,
    )
    .await;

    assert!(result.is_ok());
    let user_details = result.unwrap();
    assert_eq!(user_details.user.id, target_user.id);
    assert_eq!(user_details.user.name, "target_user");
    assert_eq!(user_details.role.name, "user");
    assert_eq!(user_details.user.role_id, user_role_id);
  }

  #[dps_auth_db_test]
  async fn test_get_user_details_without_authentication() {
    let session_context = SessionContext::new(None);

    let result =
      GetUserOrchestrator::run(&databases, session_context, &create_test_config(), 123).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("No valid session"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_user_details_without_permission() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;

    let regular_user = create_test_user_with_databases(&databases, "user1", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      regular_user.id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_user_details_nonexistent_user() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_view_user_details"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      GetUserOrchestrator::run(&databases, session_context, &create_test_config(), 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_user_details_nonexistent_session_user() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user =
      create_test_user_with_databases(&databases, "target_user", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: "999".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      target_user.id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[dps_auth_db_test]
  async fn test_get_user_details_self_access() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_view_user_details"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      admin_user.id,
    )
    .await;

    assert!(result.is_ok());
    let user_details = result.unwrap();
    assert_eq!(user_details.user.id, admin_user.id);
    assert_eq!(user_details.user.name, "admin");
    assert_eq!(user_details.role.name, "admin");
  }
}
