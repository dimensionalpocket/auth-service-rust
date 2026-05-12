use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{CheckUserPermissionService, DeleteUserService};
use crate::types::{DpsAuthApiConfig, UserError};
use crate::utils::session_context_sub_to_user_id;

pub struct DeleteUserOrchestrator;

impl DeleteUserOrchestrator {
  pub async fn run(
    databases: &Databases,
    session_context: SessionContext,
    config: &DpsAuthApiConfig,
    target_user_id: i64,
  ) -> Result<(), UserError> {
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

    let allowed = CheckUserPermissionService::run(&mut main_conn, &user, "can_delete_user").await?;

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    if user_id == target_user_id {
      return Err(UserError::SelfDeletion);
    }

    let deleted = DeleteUserService::run(&mut main_conn, target_user_id).await?;

    if !deleted {
      return Err(UserError::UserNotFound(target_user_id));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::queries::users::GetUserByIdQuery;
  use crate::test_utils::{
    create_test_config, create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test]
  async fn test_delete_user_with_permission_check_success() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_delete_user"]).await;
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

    let result = DeleteUserOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      target_user.id,
    )
    .await;

    assert!(result.is_ok());

    let deleted_user = {
      let mut main_conn = main_pool.acquire().await.unwrap();
      GetUserByIdQuery::run(&mut main_conn, target_user.id)
        .await
        .unwrap()
    };
    assert!(deleted_user.is_none());
  }

  #[dps_auth_db_test]
  async fn test_delete_user_without_authentication() {
    let session_context = SessionContext::new(None);

    let result =
      DeleteUserOrchestrator::run(&databases, session_context, &create_test_config(), 123).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("No valid session"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_delete_user_without_permission() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;

    let regular_user = create_test_user_with_databases(&databases, "user1", user_role_id).await;
    let target_user =
      create_test_user_with_databases(&databases, "target_user", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = DeleteUserOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      target_user.id,
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
  async fn test_delete_user_self_deletion_prevented() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_delete_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = DeleteUserOrchestrator::run(
      &databases,
      session_context,
      &create_test_config(),
      admin_user.id,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::SelfDeletion => {}
      _ => panic!("Expected SelfDeletion error"),
    }

    let user_still_exists = {
      let mut main_conn = main_pool.acquire().await.unwrap();
      GetUserByIdQuery::run(&mut main_conn, admin_user.id)
        .await
        .unwrap()
    };
    assert!(user_still_exists.is_some());
  }

  #[dps_auth_db_test]
  async fn test_delete_user_nonexistent_target() {
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["can_delete_user"]).await;
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id.to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result =
      DeleteUserOrchestrator::run(&databases, session_context, &create_test_config(), 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[dps_auth_db_test]
  async fn test_delete_user_nonexistent_session_user() {
    let user_role_id = create_test_role_with_databases(&databases, "user", &[]).await;
    let target_user =
      create_test_user_with_databases(&databases, "target_user", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: "999".to_string(),
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = DeleteUserOrchestrator::run(
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
}
