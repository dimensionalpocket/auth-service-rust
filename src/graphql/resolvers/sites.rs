use crate::services::SiteService;
use async_graphql::{Context, Object, Result, SimpleObject};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for site listing
#[derive(SimpleObject, Debug)]
pub struct SiteListing {
  pub id: i64,
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: String,
}

/// Sites query resolver for retrieving site information
#[derive(Default, Debug)]
pub struct SitesResolver;

#[Object]
impl SitesResolver {
  /// Returns all sites in the database.
  ///
  /// This query does not require authentication and returns basic site information
  /// including id, slug, subdomain, port, and protocol for all sites.
  ///
  /// Example response:
  /// ```json
  /// {
  ///   "sites": [
  ///     {
  ///       "id": 1,
  ///       "slug": "example",
  ///       "subdomain": "www",
  ///       "port": 443,
  ///       "protocol": "https"
  ///     }
  ///   ]
  /// }
  /// ```
  #[instrument(skip(self, ctx))]
  #[graphql(name = "sites")]
  async fn sites(&self, ctx: &Context<'_>) -> Result<Vec<SiteListing>> {
    let pool = ctx.data::<SqlitePool>()?;
    let mut conn = pool
      .acquire()
      .await
      .map_err(|_| async_graphql::Error::new("Internal server error"))?;
    let sites = SiteService::get_all_sites(&mut conn).await?;

    let site_listings: Vec<SiteListing> = sites
      .into_iter()
      .map(|site| SiteListing {
        id: site.id,
        slug: site.slug,
        subdomain: site.subdomain,
        port: site.port,
        protocol: site.protocol,
      })
      .collect();

    Ok(site_listings)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
  use crate::test_utils::{create_test_database, create_test_query_schema};

  #[tokio::test]
  async fn test_sites_empty() {
    let (pool, _tmp) = create_test_database().await;
    let query = SitesResolver;
    let schema = create_test_query_schema(query, Some(pool), None, None);

    let result = schema
      .execute("{ sites { id slug subdomain port protocol } }")
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let sites = data["sites"].as_array().unwrap();
    assert_eq!(sites.len(), 0);
  }

  #[tokio::test]
  async fn test_sites_with_data() {
    let (pool, _tmp) = create_test_database().await;

    // Create test sites
    let site1_data = CreateSiteData {
      slug: "alpha".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: None,
      metadata_json: None,
    };

    let site2_data = CreateSiteData {
      slug: "beta".to_string(),
      subdomain: None,
      port: Some(80),
      protocol: Some("http".to_string()),
      metadata_json: None,
    };

    {
      let mut conn = pool.acquire().await.unwrap();
      CreateSiteQuery::run(&mut conn, site1_data).await.unwrap();
      CreateSiteQuery::run(&mut conn, site2_data).await.unwrap();
    }

    let query = SitesResolver;
    let schema = create_test_query_schema(query, Some(pool), None, None);

    let result = schema
      .execute("{ sites { id slug subdomain port protocol } }")
      .await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let sites = data["sites"].as_array().unwrap();
    assert_eq!(sites.len(), 2);

    // Verify field structure
    assert!(sites[0]["id"].is_number());
    assert!(sites[0]["slug"].is_string());
    assert!(sites[0]["subdomain"].is_string() || sites[0]["subdomain"].is_null());
    assert!(sites[0]["port"].is_number() || sites[0]["port"].is_null());
    assert!(sites[0]["protocol"].is_string());
  }
}
