use serde::{Deserialize, Serialize};

/// Static array of all valid role permissions
pub const ROLE_PERMISSIONS: &[&str] = &[
  "is_admin",
  "can_view_user_self",
  "can_update_user_self",
  "can_create_email_self",
  "can_delete_email_self",
  "can_list_users",
  "can_view_user_details",
  "can_delete_user",
  "can_edit_user",
  "can_create_site",
  "can_delete_site",
  "can_update_site",
  "can_view_site_details",
  "can_edit_user_role",
  "can_manage_roles",
  "can_manage_admin_role_permission",
];

/// Check if a permission string is valid
pub fn is_valid_role_permission(permission: &str) -> bool {
  ROLE_PERMISSIONS.contains(&permission)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
  pub id: i64,
  pub name: String,
  pub created_ts: i64,
  pub updated_ts: i64,
  pub is_default: bool,
  pub permissions: Vec<String>,
}

impl Role {
  /// Check if this role has a specific permission
  pub fn has_permission(&self, permission: &str) -> bool {
    self.permissions.contains(&permission.to_string())
  }

  /// Deserialize permissions from JSON string to Vec<String>
  ///
  /// This is a static helper method used by query objects to convert
  /// the database representation (JSON) to the domain representation (Vec<String>).
  pub fn deserialize_permissions(json_str: &Option<String>) -> Vec<String> {
    match json_str {
      Some(json) => serde_json::from_str(json).unwrap_or_else(|_| vec![]),
      None => vec![],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_permissions_direct_access() {
    let role = Role {
      id: 1,
      name: "test".to_string(),
      created_ts: 123456789,
      updated_ts: 123456789,
      is_default: false,
      permissions: vec!["can_create_user".to_string(), "can_delete_user".to_string()],
    };

    assert_eq!(role.permissions.len(), 2);
    assert!(role.permissions.contains(&"can_create_user".to_string()));
    assert!(role.permissions.contains(&"can_delete_user".to_string()));
  }

  #[test]
  fn test_permissions_empty() {
    let role = Role {
      id: 1,
      name: "test".to_string(),
      created_ts: 123456789,
      updated_ts: 123456789,
      is_default: false,
      permissions: vec![],
    };

    assert_eq!(role.permissions.len(), 0);
  }

  #[test]
  fn test_has_permission_true() {
    let role = Role {
      id: 1,
      name: "admin".to_string(),
      created_ts: 123456789,
      updated_ts: 123456789,
      is_default: false,
      permissions: vec!["is_admin".to_string(), "can_create_user".to_string()],
    };

    assert!(role.has_permission("is_admin"));
    assert!(role.has_permission("can_create_user"));
  }

  #[test]
  fn test_has_permission_false() {
    let role = Role {
      id: 1,
      name: "user".to_string(),
      created_ts: 123456789,
      updated_ts: 123456789,
      is_default: true,
      permissions: vec!["can_view_user_self".to_string()],
    };

    assert!(!role.has_permission("is_admin"));
    assert!(!role.has_permission("can_create_user"));
    assert!(role.has_permission("can_view_user_self"));
  }

  #[test]
  fn test_has_permission_empty_permissions() {
    let role = Role {
      id: 1,
      name: "guest".to_string(),
      created_ts: 123456789,
      updated_ts: 123456789,
      is_default: false,
      permissions: vec![],
    };

    assert!(!role.has_permission("any_permission"));
  }

  #[test]
  fn test_is_valid_role_permission_valid_permissions() {
    // Test all valid permissions
    assert!(is_valid_role_permission("is_admin"));
    assert!(is_valid_role_permission("can_view_user_self"));
    assert!(is_valid_role_permission("can_update_user_self"));
    assert!(is_valid_role_permission("can_create_email_self"));
    assert!(is_valid_role_permission("can_delete_email_self"));
    assert!(is_valid_role_permission("can_list_users"));
    assert!(is_valid_role_permission("can_view_user_details"));
    assert!(is_valid_role_permission("can_delete_user"));
    assert!(is_valid_role_permission("can_edit_user"));
    assert!(is_valid_role_permission("can_create_site"));
    assert!(is_valid_role_permission("can_delete_site"));
    assert!(is_valid_role_permission("can_update_site"));
    assert!(is_valid_role_permission("can_view_site_details"));
    assert!(is_valid_role_permission("can_edit_user_role"));
    assert!(is_valid_role_permission("can_manage_roles"));
    assert!(is_valid_role_permission("can_manage_admin_role_permission"));
  }

  #[test]
  fn test_is_valid_role_permission_invalid_permissions() {
    // Test invalid permissions
    assert!(!is_valid_role_permission("invalid_permission"));
    assert!(!is_valid_role_permission("can_"));
    assert!(!is_valid_role_permission(""));
    assert!(!is_valid_role_permission("admin")); // Missing is_ prefix
    assert!(!is_valid_role_permission("CAN_VIEW_USER_SELF")); // Wrong case
    assert!(!is_valid_role_permission("can_view_user_self ")); // Trailing space
    assert!(!is_valid_role_permission(" can_view_user_self")); // Leading space
  }

  #[test]
  fn test_deserialize_permissions_valid_json() {
    let json = Some(r#"["can_create_user", "can_delete_user"]"#.to_string());
    let permissions = Role::deserialize_permissions(&json);

    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&"can_create_user".to_string()));
    assert!(permissions.contains(&"can_delete_user".to_string()));
  }

  #[test]
  fn test_deserialize_permissions_empty_json() {
    let json = Some("[]".to_string());
    let permissions = Role::deserialize_permissions(&json);

    assert_eq!(permissions.len(), 0);
  }

  #[test]
  fn test_deserialize_permissions_none() {
    let json = None;
    let permissions = Role::deserialize_permissions(&json);

    assert_eq!(permissions.len(), 0);
  }

  #[test]
  fn test_deserialize_permissions_invalid_json() {
    let json = Some("invalid json".to_string());
    let permissions = Role::deserialize_permissions(&json);

    assert_eq!(permissions.len(), 0); // Should return empty vec on parse error
  }

  #[test]
  fn test_role_permissions_array_contains_all_expected() {
    // Verify the array contains exactly the expected permissions
    let expected_permissions = vec![
      "is_admin",
      "can_view_user_self",
      "can_update_user_self",
      "can_create_email_self",
      "can_delete_email_self",
      "can_list_users",
      "can_view_user_details",
      "can_delete_user",
      "can_edit_user",
      "can_create_site",
      "can_delete_site",
      "can_update_site",
      "can_view_site_details",
      "can_edit_user_role",
      "can_manage_roles",
      "can_manage_admin_role_permission",
    ];

    assert_eq!(ROLE_PERMISSIONS.len(), expected_permissions.len());

    for expected in &expected_permissions {
      assert!(
        ROLE_PERMISSIONS.contains(expected),
        "Missing permission: {expected}"
      );
    }

    // Verify no extra permissions
    for &actual in ROLE_PERMISSIONS {
      assert!(
        expected_permissions.contains(&actual),
        "Extra permission: {actual}"
      );
    }
  }
}
