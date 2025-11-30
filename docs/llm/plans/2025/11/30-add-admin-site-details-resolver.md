# Plan: Add Admin Site Details Resolver

## Date
2025-11-30@13:56

## Overview
Create a new admin-only GraphQL resolver `site` that returns complete site data by ID, including metadata and timestamps that are not exposed in the public `sites` resolver. This resolver will be used by administrators to view full site information for management purposes.

**Naming Note**: Following GraphQL best practices, the field name will be `site` (camelCase, singular) to align with conventions from Apollo, GitLab, and other major GraphQL APIs. This maintains consistency with the existing `sites` resolver (plural for collection, singular for individual item).

## Files to Create/Modify

### 1. Create Query: `src/queries/sites/get_site_by_id.rs`
```rust
use crate::models::Site;
use sqlx::{FromRow, SqlitePool};

pub struct GetSiteByIdQuery;

impl GetSiteByIdQuery {
    pub async fn run(pool: &SqlitePool, site_id: i64) -> Result<Option<Site>, sqlx::Error> {
        let site = sqlx::query_as::<_, Site>(
            "SELECT id, created_ts, updated_ts, slug, subdomain, port, protocol, metadata_json 
             FROM sites WHERE id = ?"
        )
        .bind(site_id)
        .fetch_optional(pool)
        .await?;
        
        Ok(site)
    }
}
```

### 2. Update Query Module: `src/queries/sites/mod.rs`
Add the new query export:
```rust
pub mod create_site;
pub mod delete_site;
pub mod get_all_sites;
pub mod get_site_by_id;  // Add this line
pub mod update_site;

pub use create_site::{CreateSiteData, CreateSiteQuery};
pub use delete_site::DeleteSiteQuery;
pub use get_all_sites::GetAllSitesQuery;
pub use get_site_by_id::GetSiteByIdQuery;  // Add this line
pub use update_site::{UpdateSiteData, UpdateSiteQuery};
```

### 3. Create Resolver: `src/graphql/resolvers/site.rs`
```rust
use crate::middleware::session::SessionContext;
use crate::orchestrators::site_orchestrator::SiteOrchestrator;
use crate::services::SiteError;
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
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
    async fn site(
        &self,
        ctx: &Context<'_>,
        id: i64,
    ) -> Result<SiteDetailsResponse> {
        let pool = ctx.data::<SqlitePool>()?;
        let session_context = SessionContext::from_context(ctx)?;

        match SiteOrchestrator::get_site_details_with_permission_check(
            pool,
            session_context.clone(),
            id,
        )
        .await
        {
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
    use super::*;
    use crate::database::test_utils::create_test_database;
    use crate::middleware::session::SessionContext;
    use crate::queries::sites::{CreateSiteData, CreateSiteQuery};
    use async_graphql::*;
    use dps_auth_session::DpsAuthSessionPayload as ServiceSessionPayload;

    #[tokio::test]
    async fn test_get_site_details_success() {
        let (pool, _temp_file) = create_test_database().await;

        // Insert admin role with can_view_site_details permission
        sqlx::query(
            "INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('admin', 1234567890, FALSE, '[\"is_admin\", \"can_view_site_details\"]')"
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

        // Create test site
        let site_data = CreateSiteData {
            slug: "test-site".to_string(),
            subdomain: Some("www".to_string()),
            port: Some(443),
            protocol: Some("https".to_string()),
            metadata_json: Some("{\"description\": \"Test site\"}".to_string()),
        };
        let site = CreateSiteQuery::run(&pool, site_data).await.unwrap();

        // Create session context for admin user
        let session_payload = ServiceSessionPayload {
            sub: admin_user_id,
            iat: 1706356800,
            exp: 1706616000,
        };
        let session_context = SessionContext::new(Some(session_payload));

        let query_resolver = SiteResolver;
        let schema = Schema::build(query_resolver, EmptyMutation, EmptySubscription)
            .data(pool)
            .data(session_context)
            .finish();

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
        let site_data = &data["getSiteDetails"];

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

    #[tokio::test]
    async fn test_get_site_details_forbidden() {
        let (pool, _temp_file) = create_test_database().await;

        // Insert user role without can_view_site_details permission
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

        let query_resolver = GetSiteDetailsResolver;
        let schema = Schema::build(query_resolver, EmptyMutation, EmptySubscription)
            .data(pool)
            .data(session_context)
            .finish();

        let query = r#"
            query {
                getSiteDetails(id: 1) {
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
    async fn test_get_site_details_unauthenticated() {
        let (pool, _temp_file) = create_test_database().await;

        // Create session context with no user (unauthenticated)
        let session_context = SessionContext::new(None);

        let query_resolver = SiteResolver;
        let schema = Schema::build(query_resolver, EmptyMutation, EmptySubscription)
            .data(pool)
            .data(session_context)
            .finish();

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

    #[tokio::test]
    async fn test_get_site_details_not_found() {
        let (pool, _temp_file) = create_test_database().await;

        // Insert admin role with can_view_site_details permission
        sqlx::query(
            "INSERT INTO user_roles (name, created_ts, is_default, permissions_json) VALUES ('admin', 1234567890, FALSE, '[\"is_admin\", \"can_view_site_details\"]')"
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

        let query_resolver = SiteResolver;
        let schema = Schema::build(query_resolver, EmptyMutation, EmptySubscription)
            .data(pool)
            .data(session_context)
            .finish();

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
```

### 4. Update Resolver Module: `src/graphql/resolvers/mod.rs`
Add the new resolver to the module exports:
```rust
pub mod add_site;
pub mod auth_change_password;
pub mod auth_login;
pub mod auth_logout;
pub mod auth_me;
pub mod auth_register;
pub mod get_server_timestamp;
pub mod remove_site;
pub mod site;  // Add this line
pub mod sites;
pub mod update_site;

pub use add_site::AddSiteResolver;
pub use auth_change_password::AuthChangePasswordResolver;
pub use auth_login::AuthLoginResolver;
pub use auth_logout::AuthLogoutResolver;
pub use auth_me::AuthMeResolver;
pub use auth_register::AuthRegisterResolver;
pub use get_server_timestamp::GetServerTimestampResolver;
pub use remove_site::RemoveSiteResolver;
pub use site::SiteResolver;  // Add this line
pub use sites::SitesResolver;
pub use update_site::UpdateSiteResolver;
```

### 5. Update Site Orchestrator: `src/orchestrators/site_orchestrator.rs`
Add the new orchestrator method:
```rust
use crate::middleware::session::SessionContext;
use crate::queries::sites::{CreateSiteData, GetSiteByIdQuery, UpdateSiteData};  // Add GetSiteByIdQuery
use crate::queries::users::GetUserByIdQuery;
use crate::services::{SiteError, SiteService, UserRoleService};
use sqlx::SqlitePool;

pub struct SiteOrchestrator;

impl SiteOrchestrator {
    // ... existing methods ...

    pub async fn get_site_details_with_permission_check(
        pool: &SqlitePool,
        session_context: SessionContext,
        site_id: i64,
    ) -> Result<crate::models::Site, SiteError> {
        // Authentication: Check if user is authenticated
        let user_id = session_context
            .user_id()
            .ok_or(SiteError::AuthenticationError(
                "Authentication required".to_string(),
            ))?;

        // Authorization: Get user and check permissions
        let user = GetUserByIdQuery::run(pool, user_id)
            .await
            .map_err(SiteError::DatabaseError)?
            .ok_or(SiteError::ValidationError("User not found".to_string()))?;

        let allowed = UserRoleService::check_user_permission(pool, &user, "can_view_site_details")
            .await
            .map_err(SiteError::DatabaseError)?;

        if !allowed {
            return Err(SiteError::AuthorizationError("Forbidden".to_string()));
        }

        // Business logic: Get site details
        GetSiteByIdQuery::run(pool, site_id)
            .await
            .map_err(SiteError::DatabaseError)?
            .ok_or(SiteError::SiteNotFound(site_id))
    }
}
```

### 6. Update GraphQL Schema: `src/graphql/schema.rs`
Add the new resolver to the Query MergedObject:
```rust
use crate::graphql::resolvers::{
    AddSiteResolver, AuthChangePasswordResolver, AuthLoginResolver, AuthLogoutResolver,
    AuthMeResolver, AuthRegisterResolver, GetServerTimestampResolver, RemoveSiteResolver, SiteResolver,  // Add SiteResolver
    SitesResolver, UpdateSiteResolver,
};
use async_graphql::{EmptySubscription, MergedObject, Schema};

/// Root query object for Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL queries. It combines all
/// individual query resolvers into a single unified interface using MergedObject.
///
/// Available queries:
/// - getServerTimestamp: Get current server time for synchronization
/// - authMe: Get current authenticated user profile
/// - sites: List all sites in the database (no authentication required)
/// - site: Get complete site details by ID (admin only, requires can_view_site_details permission)
///
/// Future queries will be added here as the service expands to include
/// user authentication, profile management, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Query(GetServerTimestampResolver, AuthMeResolver, SitesResolver, SiteResolver);  // Add SiteResolver

impl Query {
  pub fn new() -> Self {
    Self::default()
  }
}

// ... rest of the file remains unchanged ...
```

### 7. Update README.md Documentation
Add the new `site` query to the Queries table in README.md. Insert this row after the `sites` row (line 33):

```markdown
| `site` | Get complete site details by ID (admin only) | id (Int!), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), createdTs (Int), updatedTs (Int) | can_view_site_details |
```

This maintains alphabetical order and documents the new admin-only query with all its fields and required permission.

## Implementation Details

### Permission Required
The resolver will require a new permission: `can_view_site_details`. This should be added to admin roles in the database seeds.

### Differences from Public Sites Resolver
- **Public `sites` resolver**: Returns limited fields (id, slug, subdomain, port, protocol) for all sites, no authentication required
- **Admin `site` resolver**: Returns complete site data (including metadata_json, created_ts, updated_ts) for a specific site, requires authentication and special permission

### Error Handling
- Authentication errors for unauthenticated users
- Authorization errors for users without `can_view_site_details` permission
- Site not found errors for non-existent site IDs
- Generic database errors for other failures

### Testing
The resolver includes comprehensive tests covering:
- Successful retrieval by admin user
- Access denied for regular users
- Access denied for unauthenticated users
- Site not found scenarios

