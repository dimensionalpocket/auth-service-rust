use crate::middleware::session::SessionContext;
use crate::queries::users::{GetUserByIdQuery, UpdateUserData};
use crate::services::{CheckUserPermissionService, UpdateUserService};
use crate::types::{user::update_user_input::UpdateUserInput, UserError};
use sqlx::SqlitePool;

pub struct UpdateUserOrchestrator;

impl UpdateUserOrchestrator {
  pub async fn run(
    pool: &SqlitePool,
    session_context: SessionContext,
    input: UpdateUserInput,
  ) -> Result<crate::models::user::UserWithRole, UserError> {
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

    let allowed = CheckUserPermissionService::run(&mut conn, &user, "can_edit_user").await?;

    if !allowed {
      return Err(UserError::AuthorizationError("Forbidden".to_string()));
    }

    let password_to_update = if input.password.is_some() || input.password_confirmation.is_some() {
      match (&input.password, &input.password_confirmation) {
        (Some(pw), Some(confirm_pw)) if pw == confirm_pw => Some(pw.clone()),
        (Some(_), None) | (None, Some(_)) => {
          return Err(UserError::ValidationError(
            "Password and password confirmation must both be provided".to_string(),
          ));
        }
        (Some(_), Some(_)) => {
          return Err(UserError::ValidationError(
            "Password and password confirmation do not match".to_string(),
          ));
        }
        _ => None,
      }
    } else {
      None
    };

    let update_data = UpdateUserData {
      id: input.id,
      name: input.name,
      role_id: input.role_id,
      password_hash: None,
      metadata_json: input.metadata_json.map(Some),
    };

    let _updated_user =
      UpdateUserService::run(&mut conn, input.id, update_data, password_to_update).await?;

    crate::queries::users::GetUserByIdWithRoleQuery::run(&mut conn, input.id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(input.id))
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{create_test_role_with_pool, create_test_user_with_pool};
  use crate::types::user::update_user_input::UpdateUserInput;
  use dps_auth_session::DpsAuthSessionPayload;

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_with_permission_check_success() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;

    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: Some("updatedname".to_string()),
      role_id: Some(admin_role_id),
      password: Some("newpassword123".to_string()),
      password_confirmation: Some("newpassword123".to_string()),
      metadata_json: Some(r#"{"updated": true}"#.to_string()),
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_ok());
    let user_with_role = result.unwrap();
    assert_eq!(user_with_role.user.name, "updatedname");
    assert_eq!(user_with_role.user.role_id, admin_role_id);
    assert_eq!(user_with_role.role.name, "admin");
    assert_eq!(
      user_with_role.user.metadata_json,
      Some(r#"{"updated": true}"#.to_string())
    );
    assert_ne!(user_with_role.user.password_hash, target_user.password_hash);
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_with_permission_check_unauthenticated() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

    let session_context = SessionContext::new(None);

    let input = UpdateUserInput {
      id: target_user.id,
      name: Some("updatedname".to_string()),
      role_id: None,
      password: None,
      password_confirmation: None,
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthenticationError(msg) => {
        assert_eq!(msg, "Authentication required");
      }
      _ => panic!("Expected AuthenticationError"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_with_permission_check_session_user_not_found() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: 999,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: Some("updatedname".to_string()),
      role_id: None,
      password: None,
      password_confirmation: None,
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_with_permission_check_forbidden() {
    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;

    let regular_user = create_test_user_with_pool(&pool, "regular", user_role_id).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: regular_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: Some("updatedname".to_string()),
      role_id: None,
      password: None,
      password_confirmation: None,
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::AuthorizationError(msg) => {
        assert_eq!(msg, "Forbidden");
      }
      _ => panic!("Expected AuthorizationError"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_with_permission_check_target_user_not_found() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: 999,
      name: Some("updatedname".to_string()),
      role_id: None,
      password: None,
      password_confirmation: None,
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_with_permission_check_validation_error() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: Some("ab".to_string()),
      role_id: None,
      password: None,
      password_confirmation: None,
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Username must be at least 3 characters long"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_with_permission_check_password_validation_error() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: None,
      role_id: None,
      password: Some("123".to_string()),
      password_confirmation: Some("123".to_string()),
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Password must be at least 6 characters long"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_with_permission_check_username_conflict() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;
    let _existing_user = create_test_user_with_pool(&pool, "existinguser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: Some("existinguser".to_string()),
      role_id: None,
      password: None,
      password_confirmation: None,
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UsernameAlreadyExists(username) => {
        assert_eq!(username, "existinguser");
      }
      _ => panic!("Expected UsernameAlreadyExists"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_password_confirmation_missing() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: None,
      role_id: None,
      password: Some("newpassword123".to_string()),
      password_confirmation: None,
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("must both be provided"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_password_confirmation_mismatch() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: None,
      role_id: None,
      password: Some("newpassword123".to_string()),
      password_confirmation: Some("different123".to_string()),
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("do not match"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_password_confirmation_match() {
    let admin_role_id = create_test_role_with_pool(&pool, "admin", &["can_edit_user"]).await;
    let admin_user = create_test_user_with_pool(&pool, "admin", admin_role_id).await;

    let user_role_id = create_test_role_with_pool(&pool, "user", &[]).await;
    let target_user = create_test_user_with_pool(&pool, "targetuser", user_role_id).await;

    let session_payload = DpsAuthSessionPayload {
      sub: admin_user.id,
      iat: 1000,
      exp: 2000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let input = UpdateUserInput {
      id: target_user.id,
      name: None,
      role_id: None,
      password: Some("newpassword123".to_string()),
      password_confirmation: Some("newpassword123".to_string()),
      metadata_json: None,
    };

    let result = UpdateUserOrchestrator::run(&pool, session_context, input).await;

    assert!(result.is_ok());
    let user_with_role = result.unwrap();
    assert_ne!(user_with_role.user.password_hash, target_user.password_hash);
  }
}
