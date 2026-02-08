use crate::middleware::session::SessionContext;
use crate::models::user::UserWithRole;
use crate::queries::users::{GetAllUsersWithRolesQuery, GetUserByIdQuery};
use crate::services::CheckUserPermissionService;
use crate::types::UserError;
use sqlx::SqlitePool;

pub struct GetUsersOrchestrator;

impl GetUsersOrchestrator {
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
  ) -> Result<Vec<UserWithRole>, UserError> {
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

    let allowed = CheckUserPermissionService::run(&mut conn, &user, "can_list_users").await?;

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    GetAllUsersWithRolesQuery::run(&mut conn)
      .await
      .map_err(UserError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{create_test_role_with_pool, create_test_user_with_pool};
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_list_users_with_permission_check_success() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_list_users"]).await;
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;

    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;
    let regular_user = create_test_user_with_pool(&pool, "user1", user_role_id).await;
    let _another_user = create_test_user_with_pool(&pool, "user2", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUsersOrchestrator::run(&pool, session_context).await;

    assert!(result.is_ok());
    let users = result.unwrap();
    assert_eq!(users.len(), 3);

    let admin_in_list = users.iter().any(|u| u.user.id == admin_user.id);
    assert!(admin_in_list);

    let user_in_list = users.iter().any(|u| u.user.id == regular_user.id);
    assert!(user_in_list);
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_list_users_without_authentication() {
    let session_context = SessionContext::new(None);

    let result = GetUsersOrchestrator::run(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert!(msg.contains("Authentication required"));
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_list_users_without_permission() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;

    let regular_user = create_test_user_with_pool(&pool, "user1", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUsersOrchestrator::run(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthorizationError(msg) => {
        assert!(msg.contains("Forbidden"));
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_list_users_nonexistent_user() {
    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUsersOrchestrator::run(&pool, session_context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]
  async fn test_list_users_empty_database() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_list_users"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let result = GetUsersOrchestrator::run(&pool, session_context).await;

    assert!(result.is_ok());
    let users = result.unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].user.id, admin_user.id);
  }
}
