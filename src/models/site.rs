use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct Site {
  pub id: i64,
  pub created_ts: i64,
  pub updated_ts: i64,
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: String,
  pub metadata_json: Option<String>,
}
