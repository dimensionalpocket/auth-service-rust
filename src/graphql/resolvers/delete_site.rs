use crate::middleware::session::SessionContext;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{SiteError, SiteService, UserRoleService};
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for site deletion response
#[derive(async_graphql::SimpleObject)]
pub struct DeleteSiteResponse {
  /// Whether deletion was successful
  pub success: bool,
  /// Success or error message
  pub message: String,
}

/// Site deletion mutation resolver
#[derive(Default, Debug)]
pub struct DeleteSiteResolver;

#[Object]
impl DeleteSiteResolver {
  /// Deletes an existing site.
  ///
  /// This mutation:
  /// - Requires user authentication
  /// - Checks if the user has "can_delete_site" permission
  /// - Deletes the site from the database
  /// - Returns success confirmation
  ///
  /// # Arguments
  /// * `site_id` - ID of the site to delete
  ///
  /// # Returns
  /// * `DeleteSiteResponse` - Success confirmation
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_delete_site" permission
  /// * Returns GraphQL error if site is not found
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(site_id = %site_id))]
  async fn delete_site(&self, ctx: &Context<'_>, site_id: i64) -> Result<DeleteSiteResponse> {
    let pool = ctx.data::<SqlitePool>()?;

    // Get session context and extract user
    let session_context = SessionContext::from_context(ctx)?;
    let user_id = session_context.user_id().ok_or("Authentication required")?;
    let user = GetUserByIdQuery::run(pool, user_id)
      .await
      .map_err(|_| "Failed to fetch user")?
      .ok_or("User not found")?;

    // Check permissions
    let allowed = UserRoleService::check_user_permission(pool, &user, "can_delete_site").await?;
    if !allowed {
      return Err(async_graphql::Error::new("Forbidden"));
    }

    match SiteService::delete_site(pool, site_id).await {
      Ok(_) => Ok(DeleteSiteResponse {
        success: true,
        message: "Site deleted successfully".to_string(),
      }),
      Err(SiteError::SiteNotFound(id)) => Err(async_graphql::Error::new(format!(
        "Site with ID {id} not found"
      ))),
      Err(err) => {
        tracing::error!("Failed to delete site: {}", err);
        Err(async_graphql::Error::new("Failed to delete site"))
      }
    }
  }
}
