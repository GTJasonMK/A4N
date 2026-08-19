mod contracts;
mod projections;
mod sessions;
mod tokens;
mod users;

pub use contracts::{
    AuthResult, BootstrapAdminInput, ChangePasswordInput, LoginInput, RefreshTokenInput,
    RegisterInput, RevokeSessionInput, SessionSummary, TokenPairSummary, UpdateProfileInput,
    UserSummary,
};
pub(crate) use projections::to_user_summary;
pub use sessions::{list_sessions, revoke_session};
pub use tokens::{
    issue_auth_result, refresh_token, user_from_access_token, verify_media_token_for_path,
};
pub use users::{
    change_user_password, get_current_user, login_user, oauth2_callback_login,
    oauth2_callback_user, register_user, require_existing_user_or_bootstrap_admin,
    update_user_profile,
};
