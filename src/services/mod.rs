pub mod auth_service;
pub mod cookie_service;
pub mod password_service;
pub mod role_service;
pub mod server_service;
pub mod session_service;
pub mod shutdown_service;
pub mod site_service;
pub mod user_service;

pub use auth_service::{AuthMeResult, AuthResult, AuthService, RegisterResult};
pub use cookie_service::CookieService;
pub use password_service::PasswordService;
pub use server_service::ServerService;
pub use session_service::SessionService;
// Re-export SessionPayload from new crate for backward compatibility
pub use dps_auth_session::DpsAuthSessionPayload as SessionPayload;
pub use role_service::RoleService;
pub use site_service::SiteService;
pub use user_service::UserService;

// Re-export error types from types module for convenience
pub use crate::types::{PasswordError, SessionError, SiteError, UserError};
