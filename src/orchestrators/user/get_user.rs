use crate::middleware::session::SessionContext;
use crate::models::user::UserWithRole;
use crate::queries::users::{GetUserByIdQuery, GetUserByIdWithRoleQuery};
use crate::services::CheckUserPermissionService;
use crate::types::UserError;
use sqlx::SqlitePool;

pub struct GetUserOrchestrator;

impl GetUserOrchestrator {
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
    target_user_id: i64,
  ) -> Result<UserWithRole, UserError> {
    let user_id = session_context
      .user_id()
      .ok_or(UserError::AuthenticationError(
        "Authentication required".to_string(),
      ))?;

    let mut conn = pool.acquire().await.map_err(UserError::DatabaseError)?;

    let user = GetUserByIdQuery::run(&mut conn, user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(user_id))?;

    let allowed =
      CheckUserPermissionService::run(&mut conn, &user, "can_view_user_details").await?;

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    GetUserByIdWithRoleQuery::run(&mut conn, target_user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(target_user_id))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{create_test_role_with_pool, create_test_user_with_pool};
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_with_permission_check_success() {
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["can_view_user_details"]).await;
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;

    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;
    let target_user = create_test_user_with_pool(&pool, "target_user", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(&pool, session_context, target_user.id).await;

    assert!(result.is_ok());
    let user_details = result.unwrap();
    assert_eq!(user_details.user.id, target_user.id);
    assert_eq!(user_details.user.name, "target_user");
    assert_eq!(user_details.role.name, "user");
    assert_eq!(user_details.user.role_id, user_role_id);
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_without_authentication() {
    let session_context = SessionContext::new(None);

    let result = GetUserOrchestrator::run(&pool, session_context, 123).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_without_permission() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;

    let regular_user = create_test_user_with_pool(&pool, "user1", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(&pool, session_context, regular_user.id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_nonexistent_user() {
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["can_view_user_details"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(&pool, session_context, 999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_nonexistent_session_user() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "target_user", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(&pool, session_context, target_user.id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_get_user_details_self_access() {
    let admin_role_id =
      create_test_role_with_pool(&pool, "admin", &["can_view_user_details"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUserOrchestrator::run(&pool, session_context, admin_user.id).await;

    assert!(result.is_ok());
    let user_details = result.unwrap();
    assert_eq!(user_details.user.id, admin_user.id);
    assert_eq!(user_details.user.name, "admin");
    assert_eq!(user_details.role.name, "admin");
  }
}
