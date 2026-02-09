use crate::models::User;
use crate::queries::users::{
  GetUserByIdQuery, GetUserByNameQuery, UpdateUserData, UpdateUserQuery,
};
use crate::services::GeneratePasswordHashService;
use crate::types::UserError;
use sqlx::SqliteConnection;

use super::{ValidateUserNameService, ValidateUserPasswordService};

pub struct UpdateUserService;

impl UpdateUserService {
  pub async fn run(
    main_conn: &mut SqliteConnection,
    user_id: i64,
    mut update_data: UpdateUserData,
    password: Option<String>,
  ) -> Result<User, UserError> {
    let current_user = GetUserByIdQuery::run(main_conn, user_id)
      .await
      .map_err(UserError::DatabaseError)?
      .ok_or(UserError::UserNotFound(user_id))?;

    if let Some(ref name) = update_data.name {
      ValidateUserNameService::run(name)?;

      if name != &current_user.name {
        if let Some(_existing_user) = GetUserByNameQuery::run(main_conn, name).await? {
          return Err(UserError::UsernameAlreadyExists(name.to_string()));
        }
      }
    }

    if let Some(password) = password {
      ValidateUserPasswordService::run(&password)?;
      let password_hash = GeneratePasswordHashService::run(&password)?;
      update_data.password_hash = Some(password_hash);
    }

    UpdateUserQuery::run(main_conn, update_data)
      .await
      .map_err(UserError::DatabaseError)
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::queries::users::{CreateUserData, CreateUserQuery};
  use crate::services::user::create_user::CreateUserService;
  use crate::test_utils::{create_test_role_model_with_databases, create_test_role_with_databases};

  #[dps_auth_db_test]
  async fn test_update_user_success_name_only() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: Some("newname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, "newname");
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, user.metadata_json);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_success_role_only() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;
    let admin_role_id = create_test_role_with_databases(&databases, "admin", &["is_admin"]).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: Some(admin_role_id),
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, admin_role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, user.metadata_json);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[dps_auth_db_test]
  async fn test_update_user_success_password_only() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(
      &mut main_conn,
      user.id,
      update_data,
      Some("newpassword123".to_string()),
    )
    .await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_ne!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, user.metadata_json);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[dps_auth_db_test]
  async fn test_update_user_success_metadata_only() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: Some(Some(r#"{"key": "value"}"#.to_string())),
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(
      updated_user.metadata_json,
      Some(r#"{"key": "value"}"#.to_string())
    );
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_success_metadata_to_null() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;
    let admin_role_id = create_test_role_with_databases(&databases, "admin", &["is_admin"]).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: Some(admin_role_id),
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: Some(r#"{"old": "data"}"#.to_string()),
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: Some(None),
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, None);
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[dps_auth_db_test(pool_size = 1)]
  async fn test_update_user_success_multiple_fields() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;
    let admin_role_id = create_test_role_with_databases(&databases, "admin", &["is_admin"]).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: Some("newname".to_string()),
      role_id: Some(admin_role_id),
      password_hash: None,
      metadata_json: Some(Some(r#"{"updated": true}"#.to_string())),
    };

    let result = UpdateUserService::run(
      &mut main_conn,
      user.id,
      update_data,
      Some("newpassword123".to_string()),
    )
    .await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, "newname");
    assert_eq!(updated_user.role_id, admin_role_id);
    assert_ne!(updated_user.password_hash, user.password_hash);
    assert_eq!(
      updated_user.metadata_json,
      Some(r#"{"updated": true}"#.to_string())
    );
    assert!(updated_user.updated_ts >= user.updated_ts);
  }

  #[dps_auth_db_test]
  async fn test_update_user_no_updates() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, user.name);
    assert_eq!(updated_user.role_id, user.role_id);
    assert_eq!(updated_user.password_hash, user.password_hash);
    assert_eq!(updated_user.metadata_json, user.metadata_json);
  }

  #[dps_auth_db_test]
  async fn test_update_user_user_not_found() {
    let update_data = UpdateUserData {
      id: 999,
      name: Some("newname".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let mut main_conn = main_pool.acquire().await.unwrap();
    let result = UpdateUserService::run(&mut main_conn, 999, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UserNotFound(user_id) => {
        assert_eq!(user_id, 999);
      }
      _ => panic!("Expected UserNotFound error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_user_username_validation_empty() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: Some("".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Username cannot be empty"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_user_username_validation_too_short() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: Some("ab".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Username must be at least 3 characters long"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_user_username_validation_too_long() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let long_name = "a".repeat(21);
    let update_data = UpdateUserData {
      id: user.id,
      name: Some(long_name.clone()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Username cannot be longer than 20 characters"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_user_username_validation_invalid_chars() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let create_data = CreateUserData {
      uuid: "test-uuid".to_string(),
      name: "testuser".to_string(),
      role_id: None,
      password_hash: GeneratePasswordHashService::run("password123").unwrap(),
      metadata_json: None,
    };
    let user = CreateUserQuery::run(&mut main_conn, create_data)
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: Some("test@user".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(
          msg.contains("Username can only contain letters, numbers, underscores, and hyphens")
        );
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_user_username_already_exists() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let user1 = CreateUserQuery::run(
      &mut main_conn,
      CreateUserData {
        uuid: "test-uuid-1".to_string(),
        name: "testuser1".to_string(),
        role_id: None,
        password_hash: GeneratePasswordHashService::run("password123").unwrap(),
        metadata_json: None,
      },
    )
    .await
    .unwrap();

    CreateUserQuery::run(
      &mut main_conn,
      CreateUserData {
        uuid: "test-uuid-2".to_string(),
        name: "testuser2".to_string(),
        role_id: None,
        password_hash: GeneratePasswordHashService::run("password123").unwrap(),
        metadata_json: None,
      },
    )
    .await
    .unwrap();

    let update_data = UpdateUserData {
      id: user1.id,
      name: Some("testuser2".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user1.id, update_data, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::UsernameAlreadyExists(username) => {
        assert_eq!(username, "testuser2");
      }
      _ => panic!("Expected UsernameAlreadyExists error"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_user_password_validation_too_short() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut main_conn, "testuser", "password123")
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(
      &mut main_conn,
      user.id,
      update_data,
      Some("123".to_string()),
    )
    .await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Password must be at least 6 characters long"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_user_password_validation_too_long() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut main_conn, "testuser", "password123")
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: None,
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result =
      UpdateUserService::run(&mut main_conn, user.id, update_data, Some("a".repeat(129))).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      UserError::ValidationError(msg) => {
        assert!(msg.contains("Password cannot be longer than 128 characters"));
      }
      _ => panic!("Expected ValidationError"),
    }
  }

  #[dps_auth_db_test]
  async fn test_update_user_same_username_no_conflict() {
    create_test_role_model_with_databases(&databases, "user", &["can_view_user_self"], true).await;

    let mut main_conn = main_pool.acquire().await.unwrap();
    let user = CreateUserService::run(&mut main_conn, "testuser", "password123")
      .await
      .unwrap();

    let update_data = UpdateUserData {
      id: user.id,
      name: Some("testuser".to_string()),
      role_id: None,
      password_hash: None,
      metadata_json: None,
    };

    let result = UpdateUserService::run(&mut main_conn, user.id, update_data, None).await;
    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.name, "testuser");
  }
}
