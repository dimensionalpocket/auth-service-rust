use sqlx::FromRow;
use serde::{Deserialize, Serialize};

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct UserRole {
  pub id: i64,
  pub name: String,
  pub created_ts: i64,
  pub is_default: bool,
  pub permissions_json: Option<String>,
}

impl UserRole {
  /// Get the permissions for this role, deserializing from JSON
  pub fn permissions(&self) -> Vec<String> {
    match &self.permissions_json {
      Some(json_str) => {
        serde_json::from_str(json_str).unwrap_or_else(|_| vec![])
      },
      None => vec![],
    }
  }

  /// Check if this role has a specific permission
  pub fn has_permission(&self, permission: &str) -> bool {
    self.permissions().contains(&permission.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_permissions_with_valid_json() {
    let role = UserRole {
      id: 1,
      name: "test".to_string(),
      created_ts: 123456789,
      is_default: false,
      permissions_json: Some(r#"["can_create_user", "can_delete_user"]"#.to_string()),
    };

    let permissions = role.permissions();
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_create_user".to_string()));
    assert!(permissions.contains(&"can_delete_user".to_string()));
  }

  #[test]
  fn test_permissions_with_empty_json() {
    let role = UserRole {
      id: 1,
      name: "test".to_string(),
      created_ts: 123456789,
      is_default: false,
      permissions_json: Some("[]".to_string()),
    };

    let permissions = role.permissions();
    assert_eq!(permissions.len(), 0);
  }

  #[test]
  fn test_permissions_with_none() {
    let role = UserRole {
      id: 1,
      name: "test".to_string(),
      created_ts: 123456789,
      is_default: false,
      permissions_json: None,
    };

    let permissions = role.permissions();
    assert_eq!(permissions.len(), 0);
  }

  #[test]
  fn test_permissions_with_invalid_json() {
    let role = UserRole {
      id: 1,
      name: "test".to_string(),
      created_ts: 123456789,
      is_default: false,
      permissions_json: Some("invalid json".to_string()),
    };

    let permissions = role.permissions();
    assert_eq!(permissions.len(), 0); // Should return empty vec on parse error
  }

  #[test]
  fn test_has_permission_true() {
    let role = UserRole {
      id: 1,
      name: "admin".to_string(),
      created_ts: 123456789,
      is_default: false,
      permissions_json: Some(r#"["is_admin", "can_create_user"]"#.to_string()),
    };

    assert!(role.has_permission("is_admin"));
    assert!(role.has_permission("can_create_user"));
  }

  #[test]
  fn test_has_permission_false() {
    let role = UserRole {
      id: 1,
      name: "user".to_string(),
      created_ts: 123456789,
      is_default: true,
      permissions_json: Some(r#"["can_view_user_self"]"#.to_string()),
    };

    assert!(!role.has_permission("is_admin"));
    assert!(!role.has_permission("can_create_user"));
    assert!(role.has_permission("can_view_user_self"));
  }

  #[test]
  fn test_has_permission_empty_permissions() {
    let role = UserRole {
      id: 1,
      name: "guest".to_string(),
      created_ts: 123456789,
      is_default: false,
      permissions_json: None,
    };

    assert!(!role.has_permission("any_permission"));
  }
}