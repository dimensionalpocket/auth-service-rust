use crate::middleware::session::SessionContext;
use crate::queries::users::GetUserByIdQuery;
use crate::services::{SiteError, SiteService, UserRoleService};
use async_graphql::{Context, Object, Result};
use sqlx::SqlitePool;
use tracing::instrument;

/// GraphQL output type for site removal response
#[derive(async_graphql::SimpleObject)]
pub struct RemoveSiteResponse {
  /// Whether removal was successful
  pub success: bool,
  /// Success or error message
  pub message: String,
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
  /// - Returns success confirmation
  ///
  /// # Arguments
  /// * `site_id` - ID of the site to remove
  ///
  /// # Returns
  /// * `RemoveSiteResponse` - Success confirmation
  ///
  /// # Errors
  /// * Returns "Authentication required" if user is not authenticated
  /// * Returns "User not found" if authenticated user doesn't exist in database
  /// * Returns "Forbidden" if user lacks "can_delete_site" permission
  /// * Returns GraphQL error if site is not found
  /// * Returns GraphQL error if database operation fails
  #[instrument(skip(ctx), fields(site_id = %site_id))]
  async fn remove_site(&self, ctx: &Context<'_>, site_id: i64) -> Result<RemoveSiteResponse> {
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
      Ok(_) => Ok(RemoveSiteResponse {
        success: true,
        message: "Site removed successfully".to_string(),
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
