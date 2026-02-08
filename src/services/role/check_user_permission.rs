use crate::models::role::is_valid_role_permission;
use crate::models::user::User;
use crate::queries::roles::GetRoleByIdQuery;
use crate::types::RoleError;
use sqlx::SqliteConnection;
use tracing::warn;

pub struct CheckUserPermissionService;

impl CheckUserPermissionService {
  pub async fn run(
    conn: &mut SqliteConnection,
    user: &User,
    permission: &str,
  ) -> Result<bool, RoleError> {
    // Validate permission exists
    if !is_valid_role_permission(permission) {
      warn!("Invalid permission checked: {}", permission);
      return Ok(false);
    }

    let role = GetRoleByIdQuery::run(&mut *conn, user.role_id)
      .await
      .map_err(RoleError::DatabaseError)?;

    match role {
      Some(role) => {
        if role.has_permission("is_admin") {
          Ok(true)
        } else {
          Ok(role.has_permission(permission))
        }
      }
      None => Ok(false),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::user::User;
  use crate::test_utils::{create_test_database, create_test_role_model_with_pool};

  #[tokio::test]
  async fn test_check_user_permission_admin_has_all_permissions() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let admin_role = create_test_role_model_with_pool(&pool, "admin", &["is_admin"], false).await;
    let admin_role_id = admin_role.id;

    let admin_user = User {
      id: 1,
      uuid: "admin-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "admin".to_string(),
      role_id: admin_role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    let mut conn = pool.acquire().await.unwrap();
    assert!(
      CheckUserPermissionService::run(&mut conn, &admin_user, "can_list_users")
        .await
        .unwrap()
    );
    assert!(
      CheckUserPermissionService::run(&mut conn, &admin_user, "can_create_site")
        .await
        .unwrap()
    );
    assert!(
      CheckUserPermissionService::run(&mut conn, &admin_user, "can_delete_user")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_regular_user_specific_permissions() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let user_role =
      create_test_role_model_with_pool(&pool, "user", &["can_view_user_self"], true).await;
    let user_role_id = user_role.id;

    let regular_user = User {
      id: 2,
      uuid: "user-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "user".to_string(),
      role_id: user_role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    let mut conn = pool.acquire().await.unwrap();
    assert!(
      CheckUserPermissionService::run(&mut conn, &regular_user, "can_view_user_self")
        .await
        .unwrap()
    );

    assert!(
      !CheckUserPermissionService::run(&mut conn, &regular_user, "can_list_users")
        .await
        .unwrap()
    );
    assert!(
      !CheckUserPermissionService::run(&mut conn, &regular_user, "is_admin")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_user_with_no_role() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let user_no_role = User {
      id: 3,
      uuid: "no-role-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "no-role".to_string(),
      role_id: 999,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    let mut conn = pool.acquire().await.unwrap();
    assert!(
      !CheckUserPermissionService::run(&mut conn, &user_no_role, "can_list_users")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_invalid_permission() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let admin_role = create_test_role_model_with_pool(&pool, "admin", &["is_admin"], false).await;
    let admin_role_id = admin_role.id;

    let admin_user = User {
      id: 1,
      uuid: "admin-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "admin".to_string(),
      role_id: admin_role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    let mut conn = pool.acquire().await.unwrap();
    assert!(
      !CheckUserPermissionService::run(&mut conn, &admin_user, "invalid_permission")
        .await
        .unwrap()
    );
    assert!(!CheckUserPermissionService::run(&mut conn, &admin_user, "")
      .await
      .unwrap());
    assert!(
      !CheckUserPermissionService::run(&mut conn, &admin_user, "nonexistent_can_permission")
        .await
        .unwrap()
    );
  }

  #[tokio::test]
  async fn test_check_user_permission_valid_new_permissions() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let role = create_test_role_model_with_pool(
      &pool,
      "role_manager",
      &["can_edit_user_role", "can_manage_roles"],
      false,
    )
    .await;
    let role_id = role.id;

    let role_manager_user = User {
      id: 2,
      uuid: "role-manager-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "role_manager".to_string(),
      role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    let mut conn = pool.acquire().await.unwrap();
    assert!(
      CheckUserPermissionService::run(&mut conn, &role_manager_user, "can_edit_user_role")
        .await
        .unwrap()
    );
    assert!(
      CheckUserPermissionService::run(&mut conn, &role_manager_user, "can_manage_roles")
        .await
        .unwrap()
    );
    assert!(
      !CheckUserPermissionService::run(&mut conn, &role_manager_user, "can_list_users")
        .await
        .unwrap()
    );
    assert!(
      CheckUserPermissionService::run(&mut conn, &role_manager_user, "can_manage_roles")
        .await
        .unwrap()
    );

    assert!(!CheckUserPermissionService::run(
      &mut conn,
      &role_manager_user,
      "can_manage_admin_role_permission"
    )
    .await
    .unwrap());
  }

  #[tokio::test]
  async fn test_check_user_permission_admin_bypasses_validation() {
    let (databases, _main_temp_file, _session_temp_file) = create_test_database().await;
    let pool = databases.main().clone();

    let admin_role = create_test_role_model_with_pool(&pool, "admin", &["is_admin"], false).await;
    let admin_role_id = admin_role.id;

    let admin_user = User {
      id: 1,
      uuid: "admin-uuid".to_string(),
      created_ts: 1234567890,
      updated_ts: 1234567890,
      name: "admin".to_string(),
      role_id: admin_role_id,
      password_hash: "hash".to_string(),
      metadata_json: None,
    };

    let mut conn = pool.acquire().await.unwrap();
    assert!(
      CheckUserPermissionService::run(&mut conn, &admin_user, "can_edit_user_role")
        .await
        .unwrap()
    );
    assert!(
      CheckUserPermissionService::run(&mut conn, &admin_user, "can_manage_roles")
        .await
        .unwrap()
    );
    assert!(CheckUserPermissionService::run(
      &mut conn,
      &admin_user,
      "can_manage_admin_role_permission"
    )
    .await
    .unwrap());
  }
}
