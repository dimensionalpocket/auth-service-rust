use async_graphql::SimpleObject;

#[derive(SimpleObject, Debug)]
pub struct SiteListing {
  pub id: i64,
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: String,
}
