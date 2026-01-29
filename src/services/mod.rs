pub mod auth;
pub mod cookie;
pub mod password;
pub mod role;
pub mod server;
pub mod session;
pub mod shutdown;
pub mod site;
pub mod user;

pub use auth::AuthGetCurrentUserService;
pub use auth::AuthLoginService;
pub use auth::AuthMeResult;
pub use auth::AuthRegisterService;
pub use auth::AuthResult;
pub use auth::RegisterResult;
pub use cookie::GenerateLogoutCookieService;
pub use cookie::GenerateSessionCookieService;
pub use password::GeneratePasswordHashService;
pub use password::VerifyPasswordService;
pub use role::CheckUserPermissionService;
pub use role::CreateRoleService;
pub use role::DeleteRoleService;
pub use role::GetAllRolesService;
pub use role::GetRoleByIdService;
pub use role::GetRoleByNameService;
pub use role::SetDefaultRoleService;
pub use role::UpdateRoleService;
pub use server::GetServerTimestampService;
pub use session::CreateSessionForUserService;
pub use session::CreateSessionService;
pub use shutdown::LogShutdownStartService;
pub use shutdown::WaitForShutdownSignalService;
pub use site::CreateSiteService;
pub use site::DeleteSiteService;
pub use site::GetAllSitesService;
pub use site::UpdateSiteService;
pub use site::ValidateSiteSlugService;
pub use user::CreateUserService;
pub use user::DeleteUserService;
pub use user::GetUserByIdService;
pub use user::GetUserByNameService;
pub use user::UpdateUserPasswordService;
pub use user::UpdateUserService;
pub use user::ValidateUserNameService;
pub use user::ValidateUserPasswordService;

// Re-export SessionPayload from new crate for backward compatibility
pub use dps_auth_session::DpsAuthSessionPayload as SessionPayload;

// Re-export error types from types module for convenience
pub use crate::types::{PasswordError, SessionError, SiteError, UserError};
