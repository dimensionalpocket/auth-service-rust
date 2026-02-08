use crate::middleware::session::SessionContext;
use crate::orchestrators::site::RemoveSiteOrchestrator;
use crate::types::SiteError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for site removal response
#[derive(async_graphql::SimpleObject)]
pub struct RemoveSiteResponse {
  /// The deleted site's database ID
  pub id: i64,
  /// The deleted site's unique slug
  pub slug: String,
  /// The deleted site's subdomain (if any)
  pub subdomain: Option<String>,
  /// The deleted site's port (if any)
  pub port: Option<i64>,
  /// The deleted site's protocol
  pub protocol: String,
  /// The deleted site's metadata JSON (if any)
  #[graphql(name = "metadataJson")]
  pub metadata_json: Option<String>,
  /// Timestamp when the site was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the site was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// Site removal mutation resolver
#[derive(Default, Debug)]
pub struct RemoveSiteResolver;

#[Object]
impl RemoveSiteResolver {
  /// Removes an existing site.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if the user has "can_delete_site" permission
  /// - Removes the site from the database
  /// - Returns the deleted site information
  ///
  /// # Arguments
  /// * `site_id` - ID of the site to remove
  ///
  /// # Returns
  /// * `RemoveSiteResponse` - The deleted site information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_delete_site" permission
  /// * Returns GraphQL error if site is not found
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(site_id = %site_id))]
  #[graphql(name = "removeSite")]
  async fn remove_site(
    &self,
    ctx: &Context<'_>,
    #[graphql(name = "siteId")] site_id: i64,
  ) -> Result<RemoveSiteResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    match RemoveSiteOrchestrator::run(pool, session_context.clone(), site_id).await {
      Ok(site) => Ok(RemoveSiteResponse {
        id: site.id,
        slug: site.slug,
        subdomain: site.subdomain,
        port: site.port,
        protocol: site.protocol,
        metadata_json: site.metadata_json,
        created_ts: site.created_ts,
        updated_ts: site.updated_ts,
      }),
      Err(SiteError::SiteNotFound(id)) => Err(async_graphql::Error::new(format!(
        "Site with ID {id} not found"
      ))),
      Err(SiteError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(SiteError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to delete site: {}", err);
        Err(async_graphql::Error::new("Failed to delete site"))
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
    create_test_mutation_schema, create_test_role_with_pool, create_test_user_with_pool,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[dps_auth_db_test]
  async fn test_remove_site_success() {
    // Create admin role with can_delete_site permission
    let admin_role_id =
      create_test_role_with_pool(&main_pool, "admin", &["is_admin", "can_delete_site"]).await;

    // Create admin user
    let admin_user = create_test_user_with_pool(&main_pool, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create a site first
    let create_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some(r#"{"description": "Test site"}"#.to_string()),
    };
    let site = {
      let mut conn = main_pool.acquire().await.unwrap();
      CreateSiteQuery::run(&mut conn, create_data).await.unwrap()
    };

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = RemoveSiteResolver;
    let schema =
      create_test_mutation_schema(mutation, Some(main_pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeSite(siteId: $SITE_ID) {
          id
          slug
          subdomain
          port
          protocol
          metadataJson
          createdTs
          updatedTs
        }
      }
    "#
    .replace("$SITE_ID", &site.id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let site_data = &data["removeSite"];

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
  async fn test_remove_site_forbidden() {
    // Create user role without can_delete_site permission
    let user_role_id =
      create_test_role_with_pool(&main_pool, "user", &["can_view_user_self"]).await;

    // Create regular user
    let user = create_test_user_with_pool(&main_pool, "user", user_role_id).await;
    let user_id = user.id;

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = RemoveSiteResolver;
    let schema =
      create_test_mutation_schema(mutation, Some(main_pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeSite(siteId: 1) {
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
  async fn test_remove_site_unauthenticated() {
    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let mutation = RemoveSiteResolver;
    let schema =
      create_test_mutation_schema(mutation, Some(main_pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeSite(siteId: 1) {
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
  async fn test_remove_site_not_found() {
    // Create admin role with can_delete_site permission
    let admin_role_id =
      create_test_role_with_pool(&main_pool, "admin", &["is_admin", "can_delete_site"]).await;

    // Create admin user
    let admin_user = create_test_user_with_pool(&main_pool, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = RemoveSiteResolver;
    let schema =
      create_test_mutation_schema(mutation, Some(main_pool), Some(session_context), None);

    let query = r#"
      mutation {
        removeSite(siteId: 999) {
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
