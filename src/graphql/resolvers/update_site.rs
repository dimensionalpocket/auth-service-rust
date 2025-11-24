use crate::middleware::session::SessionContext;
use crate::orchestrators::site_orchestrator::SiteOrchestrator;
use crate::queries::sites::UpdateSiteData;
use crate::services::SiteError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for site update response
#[derive(async_graphql::SimpleObject)]
pub struct UpdateSiteResponse {
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

/// Site update mutation resolver
#[derive(Default, Debug)]
pub struct UpdateSiteResolver;

#[Object]
impl UpdateSiteResolver {
  /// Updates an existing site with the provided parameters.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if the user has "can_update_site" permission
  /// - Validates the slug format and uniqueness if provided
  /// - Updates only the fields provided (PATCH semantics)
  /// - Explicit null values set database columns to NULL
  /// - Automatically updates the updated_ts timestamp
  /// - Returns the updated site information
  ///
  /// # Arguments
  /// * `id` - Site ID to update
  /// * `slug` - Optional new slug for the site (3-20 chars, alphanumeric + underscore/hyphen)
  /// * `subdomain` - Optional new subdomain for the site
  /// * `port` - Optional new port number for the site
  /// * `protocol` - Optional new protocol for the site
  /// * `metadata_json` - Optional new JSON metadata for the site
  ///
  /// # Returns
  /// * `UpdateSiteResponse` - The updated site information
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_update_site" permission
  /// * Returns GraphQL error if slug validation fails
  /// * Returns GraphQL error if slug already exists
  /// * Returns GraphQL error if site is not found
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(id = %id))]
  #[allow(clippy::too_many_arguments)]
  #[graphql(name = "updateSite")]
  async fn update_site(
    &self,
    ctx: &Context<'_>,
    id: i64,
    slug: Option<String>,
    subdomain: Option<String>,
    port: Option<i64>,
    protocol: Option<String>,
    #[graphql(name = "metadataJson")] metadata_json: Option<String>,
  ) -> Result<UpdateSiteResponse> {
    let pool = ctx.data::<SqlitePool>()?;

    // Get session context
    let session_context = SessionContext::from_context(ctx)?;

    // Convert parameters to UpdateSiteData with proper null handling
    let update_data = UpdateSiteData {
      id,
      slug,
      subdomain: subdomain.map(Some), // Convert Option<T> to Option<Option<T>>
      port: port.map(Some),
      protocol,
      metadata_json: metadata_json.map(Some),
    };

    match SiteOrchestrator::update_site_with_permission_check(
      pool,
      session_context.clone(),
      id,
      update_data,
    )
    .await
    {
      Ok(Some(site)) => Ok(UpdateSiteResponse {
        id: site.id,
        slug: site.slug,
        subdomain: site.subdomain,
        port: site.port,
        protocol: site.protocol,
        metadata_json: site.metadata_json,
        created_ts: site.created_ts,
        updated_ts: site.updated_ts,
      }),
      Ok(None) => Err(async_graphql::Error::new("Site not found")),
      Err(SiteError::SlugAlreadyExists(slug)) => Err(async_graphql::Error::new(format!(
        "Slug '{slug}' is already in use"
      ))),
      Err(SiteError::ValidationError(msg)) => Err(async_graphql::Error::new(format!(
        "Validation error: {msg}"
      ))),
      Err(SiteError::SiteNotFound(id)) => Err(async_graphql::Error::new(format!(
        "Site with ID {id} not found"
      ))),
      Err(SiteError::AuthenticationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(SiteError::AuthorizationError(msg)) => Err(async_graphql::Error::new(msg)),
      Err(err) => {
        tracing::error!("Failed to update site: {}", err);
        Err(async_graphql::Error::new("Failed to update site"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::middleware::session::SessionContext;
  use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
  use async_graphql::*;
  use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

  // Minimal query struct for testing mutations in isolation
  #[derive(Default)]
  struct TestEmptyQuery;

  #[Object]
  impl TestEmptyQuery {
    async fn dummy(&self) -> &str {
      "test"
    }
  }

  #[tokio::test]
  async fn test_update_site_success() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_update_site permission
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('admin', 1234567890, FALSE, '[\"is_admin\", \"can_update_site\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create a site first
    let create_data = CreateSiteData {
      slug: "test-site".to_string(),
      subdomain: Some("www".to_string()),
      port: Some(443),
      protocol: Some("https".to_string()),
      metadata_json: Some(r#"{"description": "Test site"}"#.to_string()),
    };
    let site = CreateSiteQuery::run(&pool, create_data).await.unwrap();

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateSiteResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateSite(
          id: $SITE_ID,
          slug: "updated-site", 
          subdomain: "api", 
          port: 8080, 
          protocol: "http",
          metadataJson: "{\"updated\": true}"
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
    "#
    .replace("$SITE_ID", &site.id.to_string());

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    let site_data = &data["updateSite"];

    assert_eq!(site_data["id"].as_i64().unwrap(), site.id);
    assert_eq!(site_data["slug"].as_str().unwrap(), "updated-site");
    assert_eq!(site_data["subdomain"].as_str().unwrap(), "api");
    assert_eq!(site_data["port"].as_i64().unwrap(), 8080);
    assert_eq!(site_data["protocol"].as_str().unwrap(), "http");
    assert_eq!(
      site_data["metadataJson"].as_str().unwrap(),
      "{\"updated\": true}"
    );
    assert!(site_data["createdTs"].as_i64().unwrap() > 0);
    assert!(site_data["updatedTs"].as_i64().unwrap() > 0);
  }

  #[tokio::test]
  async fn test_update_site_forbidden() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert user role without can_update_site permission
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('user', 1234567890, TRUE, '[\"can_view_user_self\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert regular user
    let user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('user-uuid', 1234567890, 1234567890, 'user', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let user_id = user_result.last_insert_rowid();

    // Create session context for regular user
    let session_payload = ServiceSessionPayload {
      sub: user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateSiteResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateSite(id: 1, slug: "forbidden-site") {
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
  async fn test_update_site_unauthenticated() {
    let (pool, _temp_file) = create_test_database().await;

    // Create session context with no user (unauthenticated)
    let session_context = SessionContext::new(None);

    let mutation = UpdateSiteResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateSite(id: 1, slug: "unauth-site") {
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
  async fn test_update_site_not_found() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_update_site permission
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('admin', 1234567890, FALSE, '[\"is_admin\", \"can_update_site\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateSiteResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateSite(id: 999, slug: "nonexistent-site") {
          id
          slug
        }
      }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("not found"));
  }

  #[tokio::test]
  async fn test_update_site_invalid_slug() {
    let (pool, _temp_file) = create_test_database().await;

    // Insert admin role with can_update_site permission
    sqlx::query(
      "INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('admin', 1234567890, FALSE, '[\"is_admin\", \"can_update_site\"]')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert admin user
    let admin_user_result = sqlx::query(
      "INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('admin-uuid', 1234567890, 1234567890, 'admin', 1, 'hashed_password', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_user_id = admin_user_result.last_insert_rowid();

    // Create a site first
    let create_data = CreateSiteData {
      slug: "valid-site".to_string(),
      subdomain: None,
      port: None,
      protocol: None,
      metadata_json: None,
    };
    let site = CreateSiteQuery::run(&pool, create_data).await.unwrap();

    // Create session context for admin user
    let session_payload = ServiceSessionPayload {
      sub: admin_user_id,
      iat: 1706356800,
      exp: 1706616000,
    };
    let session_context = SessionContext::new(Some(session_payload));

    let mutation = UpdateSiteResolver;
    let schema = Schema::build(TestEmptyQuery, mutation, EmptySubscription)
      .data(pool)
      .data(session_context)
      .finish();

    let query = r#"
      mutation {
        updateSite(
          id: $SITE_ID,
          slug: "ab"
        ) {
          id
          slug
        }
      }
    "#
    .replace("$SITE_ID", &site.id.to_string());

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Validation error"));
  }
}
