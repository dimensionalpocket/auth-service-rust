use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct User {
  pub id: i64,
  pub uuid: String,
  pub created_ts: i64,
  pub updated_ts: i64,
  pub name: String,
  pub role_id: i64,
  pub password_hash: String,
  pub metadata_json: Option<String>,
}