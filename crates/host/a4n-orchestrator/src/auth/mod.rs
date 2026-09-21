//! 认证公共类型与纯规则。
//! DB 行为由 `context/auth.rs` 的私有 `AuthApplication` 持有。

mod rules;

pub(crate) use a4n_api_contract::auth::{required_session_id, required_session_user_id};
pub(crate) use rules::ensure_same_user;
