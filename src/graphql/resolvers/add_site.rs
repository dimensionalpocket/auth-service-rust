use crate::middleware::session::SessionContext;
use crate::orchestrators::site_orchestrator::SiteOrchestrator;
use crate::queries::sites::CreateSiteData;
use crate::services::SiteError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for site addition response
#[derive(async_graphql::SimpleObject)]
pub struct AddSiteResponse {
  /// The created site's database ID
  pub id: i64,
  /// The site's unique slug
  pub slug: String,
  /// The site's subdomain (if any)
  pub subdomain: Option<String>,
  /// The site's port (if any)
  pub port: Option<i64>,
  /// The site's protocol
  pub protocol: String,
  /// The site's metadata JSON (if any)
  #[graphql(name = "metadataJson")]
  pub metadata_json: Option<String>,
  /// Timestamp when the site was created
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  /// Timestamp when the site was last updated
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
}

/// Site addition mutation resolver
#[derive(Default, Debug)]
pub struct AddSiteResolver;

#[Object]
impl AddSiteResolver {
  /// Adds a new site with the provided parameters.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if the user has "can_create_site" permission
  /// - Validates the slug format and uniqueness
  /// - Creates the site in the database
  /// - Returns the created site information
  ///
  /// # Arguments
  /// * `slug` - Unique slug identifier for site (3-20 chars, alphanumeric + underscore/hyphen)
  /// * `subdomain` - Optional subdomain for the site
  /// * `port` - Optional port number for the site
  /// * `protocol` - Protocol (defaults to "https" if not specified)
  /// * `metadata_json` - Optional JSON metadata for the site
  ///
  /// # Returns
  /// * `AddSiteResponse` - The created site information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_create_site" permission
  /// * Returns GraphQL error if slug validation fails
  /// * Returns GraphQL error if slug already exists
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(slug = %slug))]
  #[graphql(name = "addSite")]
  async fn add_site(
    &self,
    ctx: &Context<'_>,
    slug: String,
    subdomain: Option<String>,
    port: Option<i64>,
    protocol: Option<String>,
    #[graphql(name = "metadataJson")] metadata_json: Option<String>,
  ) -> Result<AddSiteResponse> {
    let pool = ctx.data::<SqlitePool>()?;
    let session_context = SessionContext::from_context(ctx)?;

    let create_data = CreateSiteData {
      slug,
      subdomain,
      port,
      protocol,
      metadata_json,
    };

    match SiteOrchestrator::create_site_with_permission_check(
      pool,
      session_context.clone(),
      create_data,
    )
    .await
    {
      Ok(site) => Ok(AddSiteResponse {
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
      Err(SiteError::SlugAlreadyExists(slug)) => Err(async_graphql::Error::new(format!(
        "Slug '{slug}' is already in use"
      ))),
      Err(SiteError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(err) => {
        tracing::error!("Failed to create site: {}", err);
        Err(async_graphql::Error::new("Failed to create site"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::middleware::session::SessionContext;
  use crate::test_utils::{
    create_test_database, create_test_mutation_schema, create_test_role, create_test_user,
  };
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  #[tokio::test]
  async fn test_add_site_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role with can_create_site permission
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_create_site"]).await;

    // Create admin user
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = AddSiteResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addSite(
          slug: "test-site", 
          subdomain: "www", 
          port: 443, 
          protocol: "https",
          metadataJson: "{\"description\": \"Test site\"}"
        ) {
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
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let site_data = &data["addSite"];

    assert!(site_data["id"].as_i64().unwrap() > 0);
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

  #[tokio::test]
  async fn test_add_site_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Create user role without can_create_site permission
    let user_role_id = create_test_role(&pool, "user", &["can_view_user_self"]).await;

    // Create regular user
    let user = create_test_user(&pool, "user", user_role_id).await;
    let user_id = user.id;

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = AddSiteResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addSite(slug: "forbidden-site") {
          id
          slug
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Forbidden"));
  }

  #[tokio::test]
  async fn test_add_site_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let mutation = AddSiteResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addSite(slug: "unauth-site") {
          id
          slug
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Authentication required"));
  }

  #[tokio::test]
  async fn test_add_site_duplicate_slug() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role with can_create_site permission
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_create_site"]).await;

    // Create admin user
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = AddSiteResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    let query = r#"
      mutation {
        addSite(slug: "duplicate") {
          id
          slug
        }
      }
    "#;

    // First creation should succeed
    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    // Second creation with same slug should fail
    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("already in use"));
  }

  #[tokio::test]
  async fn test_add_site_invalid_slug() {
    let (pool, _temp_file) = create_test_database().await;

    // Create admin role with can_create_site permission
    let admin_role_id = create_test_role(&pool, "admin", &["is_admin", "can_create_site"]).await;

    // Create admin user
    let admin_user = create_test_user(&pool, "admin", admin_role_id).await;
    let admin_user_id = admin_user.id;

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = AddSiteResolver;
    let schema = create_test_mutation_schema(mutation, Some(pool), Some(session_context), None);

    // Test slug that's too short
    let query = r#"
      mutation {
        addSite(slug: "ab") {
          id
          slug
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Validation error"));
  }
}
