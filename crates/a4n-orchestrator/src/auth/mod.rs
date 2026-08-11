//! 认证公共类型与纯规则。
//! DB 行为由 `context/auth.rs` 的私有 `AuthApplication` 持有。

mod rules;

pub use a4n_db::auth::BootstrapAdminInput;
pub use rules::{
    ensure_same_user, oauth2_state, required_authorization_code, required_session_id,
    required_session_user_id,
};
