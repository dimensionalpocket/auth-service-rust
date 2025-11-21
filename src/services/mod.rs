pub mod auth_service;
pub mod password_service;
pub mod server_service;
pub mod session_service;
pub mod shutdown_service;
pub mod site_service;
pub mod user_role_service;
pub mod user_service;

pub use auth_service::{AuthMeResult, AuthResult, AuthService, RegisterResult};
pub use password_service::{PasswordError, PasswordService};
pub use server_service::ServerService;
pub use session_service::{SessionError, SessionService};
// Re-export SessionPayload from the new crate for backward compatibility
pub use dps_auth_session::DpsAuthSessionPayload as SessionPayload;
pub use site_service::{SiteError, SiteService};
pub use user_role_service::UserRoleService;
pub use user_service::{UserError, UserService};
