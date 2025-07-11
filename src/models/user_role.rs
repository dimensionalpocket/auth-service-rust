use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct UserRole {
  pub id: i64,
  pub name: String,
  pub created_ts: i64,
  pub is_default: bool,
}