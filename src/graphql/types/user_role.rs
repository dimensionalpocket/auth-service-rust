use crate::models::Role;
use async_graphql::{Object, ID};

pub struct UserRole {
  pub id: ID,
  pub name: String,
  pub permissions: Vec<String>,
}

#[Object]
impl UserRole {
  pub async fn id(&self) -> ID {
    self.id.clone()
  }

  pub async fn name(&self) -> &str {
    &self.name
  }

  pub async fn permissions(&self) -> &Vec<String> {
    &self.permissions
  }
}

impl From<Role> for UserRole {
  fn from(role: Role) -> Self {
    Self {
      id: ID(role.id.to_string()),
      name: role.name,
      permissions: role.permissions, // Direct access to Vec<String>
    }
  }
}
