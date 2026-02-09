use crate::database::Databases;
use crate::middleware::session::SessionContext;
use crate::orchestrators::site::GetSiteOrchestrator;
use crate::types::SiteError;
use async_graphql::{Context, Object, Result};
use tracing::instrument;

/// GraphQL output type for complete site details (admin only)
#[derive(async_graphql::SimpleObject)]
pub struct SiteDetailsResponse {
  /// The site's database ID
  pub id: i64,
  /// The site's unique slug
  pub slug: String,
  /// The site's subdomain (if any)
  pub subdomain: Option<String>,
  /// The site's port (if any)
  pub port: Option<i64>,
  /// The site's protocol
  pub protocol: String,
  /// The site's metadata JSON (if any) - admin only field
  #[graphql(name = "metadataJson")]
  pub metadata_json: Option<String>,
  /// Timestamp when the site was created - admin only field
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the site was last updated - admin only field
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// Site query resolver (admin only) - returns complete site data by ID
#[derive(Default, Debug)]
pub struct SiteResolver;

#[Object]
impl SiteResolver {
  /// Returns complete site information by ID (admin only).
  ///
  /// This query:
  /// - Requires user authentication
  /// - Checks if the user has "can_view_site_details" permission
  /// - Returns all site fields including metadata and timestamps
  /// - Returns error if site is not found
  ///
  /// # Arguments
  /// * `id` - The database ID of the site to retrieve
  ///
  /// # Returns
  /// * `SiteDetailsResponse` - Complete site information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_view_site_details" permission
  /// * Returns "Site not found" if site with given ID doesn't exist
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(site_id = %id))]
  #[graphql(name = "site")]
  async fn site(&self, ctx: &Context<'_>, id: i64) -> Result<SiteDetailsResponse> {
    let databases = ctx.data::<Databases>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match GetSiteOrchestrator::run(databases, session_context.clone(), id).await {
      Ok(site) => Ok(SiteDetailsResponse {
        id: site.id,
        slug: site.slug,
        subdomain: site.subdomain,
        port: site.port,
        protocol: site.protocol,
        metadata_json: site.metadata_json,
        created_ts: site.created_ts,
        updated_ts: site.updated_ts,
      }),
      Err(SiteError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(SiteError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(SiteError::SiteNotFound(site_id)) => Err(async_graphql::Error::new(format!(
        "Site with ID {site_id} not found"
      ))),
      Err(err) => {
        tracing::error!("Failed to get site details: {}", err);
        Err(async_graphql::Error::new("Failed to retrieve site details"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use dps_auth_test_macros::dps_auth_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
  use crate::test_utils::{
    create_test_query_schema, create_test_role_with_databases, create_test_user_with_databases,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[dps_auth_db_test]
  async fn test_get_site_details_success() {
    // Create admin role with can_view_site_details permission
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_view_site_details"])
        .await;

    // Create admin user
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create test site
    let site_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some("{\"description\": \"Test site\"}".to_string()),
    };
    let site = {
      let mut main_conn = main_pool.acquire().await.unwrap();
      CreateSiteQuery::run(&mut main_conn, site_data)
        .await
        .unwrap()
    };

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = SiteResolver;
    let schema = create_test_query_schema(
      query_resolver,
      databases.clone(),
      Some(session_context),
      None,
    );

    let query = format!(
      r#"
            query {{
                site(id: {}) {{
                    id
                    slug
                    subdomain
                    port
                    protocol
                    metadataJson
                    createdTs
                    updatedTs
                }}
            }}
            "#,
      site.id
    );

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let site_data = &data["site"];

    assert_eq!(site_data["id"].as_i64().unwrap(), site.id);
    assert_eq!(site_data["slug"].as_str().unwrap(), "test-site");
    assert_eq!(site_data["subdomain"].as_str().unwrap(), "www");
    assert_eq!(site_data["port"].as_i64().unwrap(), 443);
    assert_eq!(site_data["protocol"].as_str().unwrap(), "https");
    assert_eq!(
      site_data["metadataJson"].as_str().unwrap(),
      "{\"description\": \"Test site\"}"
    );
    assert!(site_data["createdTs"].as_i64().unwrap() > 0);
    assert!(site_data["updatedTs"].as_i64().unwrap() > 0);
  }

  #[dps_auth_db_test]
  async fn test_get_site_details_forbidden() {
    // Create user role without can_view_site_details permission
    let user_role_id =
      create_test_role_with_databases(&databases, "user", &["can_view_user_self"]).await;

    // Create regular user
    let user = create_test_user_with_databases(&databases, "user", user_role_id).await;
    let user_id = user.id;

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = SiteResolver;
    let schema = create_test_query_schema(
      query_resolver,
      databases.clone(),
      Some(session_context),
      None,
    );

    let query = r#"
            query {
                site(id: 1) {
                    id
                    slug
                }
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[dps_auth_db_test]
  async fn test_get_site_details_unauthenticated() {
    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let query_resolver = SiteResolver;
    let schema = create_test_query_schema(
      query_resolver,
      databases.clone(),
      Some(session_context),
      None,
    );

    let query = r#"
            query {
                site(id: 1) {
                    id
                    slug
                }
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }

  #[dps_auth_db_test]
  async fn test_get_site_details_not_found() {
    // Create admin role with can_view_site_details permission
    let admin_role_id =
      create_test_role_with_databases(&databases, "admin", &["is_admin", "can_view_site_details"])
        .await;

    // Create admin user
    let admin_user = create_test_user_with_databases(&databases, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let query_resolver = SiteResolver;
    let schema = create_test_query_schema(
      query_resolver,
      databases.clone(),
      Some(session_context),
      None,
    );

    let query = r#"
            query {
                site(id: 999) {
                    id
                    slug
                }
            }
        "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("not found"));
  }
}
