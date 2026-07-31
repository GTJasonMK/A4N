use serde_json::Value;

use super::super::super::{ApiError, ServerState, auth_records_actions, required_arg};

const SYSTEM_DEFAULT_CONFIG_ID: i64 = -1;

pub(in crate::routes::commands::front) async fn require_global_config_admin(
    state: &ServerState,
    user_id: i64,
) -> Result<(), ApiError> {
    let user = auth_records_actions::get_current_user(state.runtime(), user_id).await?;
    if !user.is_active {
        return Err(ApiError::forbidden(format!("用户 {user_id} 已停用")));
    }
    let role = a4n_core::models::user::role::parse_user_role(&user.role)?;
    if role == a4n_core::models::user::role::UserRole::Admin {
        Ok(())
    } else {
        Err(ApiError::forbidden("只有管理员可以修改全局 API 配置"))
    }
}

pub(super) fn i64_arg_or_system_default(args: &Value, key: &str) -> Result<i64, ApiError> {
    let value = required_arg(args, key)?
        .as_i64()
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是整数")))?;
    if value == SYSTEM_DEFAULT_CONFIG_ID || value > 0 {
        Ok(value)
    } else {
        Err(ApiError::bad_request(format!(
            "{key} 必须是正整数或系统默认配置 ID"
        )))
    }
}

macro_rules! config_web_no_arg_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            _user_id: i64,
            _args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            $crate::routes::commands::to_value(
                $action(state.runtime())
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! config_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            super::shared::require_global_config_admin(state, user_id).await?;
            let input: $input = $crate::routes::commands::command_input(args, stringify!($name))?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), input)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! config_web_update_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            super::shared::require_global_config_admin(state, user_id).await?;
            let config_id = $crate::routes::commands::i64_arg(args, "configId")?;
            let input: $input = $crate::routes::commands::command_input(args, stringify!($name))?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), config_id, input)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! config_web_id_value_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            _user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let config_id = $crate::routes::commands::i64_arg(args, "configId")?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), config_id)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! config_web_admin_id_value_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            super::shared::require_global_config_admin(state, user_id).await?;
            let config_id = $crate::routes::commands::i64_arg(args, "configId")?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), config_id)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! config_web_id_unit_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            super::shared::require_global_config_admin(state, user_id).await?;
            let config_id = $crate::routes::commands::i64_arg(args, "configId")?;
            $action(state.runtime(), config_id)
                .await
                .map_err($crate::routes::commands::ApiError::bad_request)?;
            Ok(serde_json::Value::Null)
        }
    };
}

macro_rules! config_web_model_discovery_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let input: $crate::routes::commands::config_records_actions::ModelDiscoveryInput =
                $crate::routes::commands::command_input(args, stringify!($name))?;
            if input.use_saved_api_key {
                super::shared::require_global_config_admin(state, user_id).await?;
            }
            $crate::routes::commands::to_value(
                $action(state.runtime(), input)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

pub(super) use config_web_admin_id_value_command;
pub(super) use config_web_id_unit_command;
pub(super) use config_web_id_value_command;
pub(super) use config_web_input_command;
pub(super) use config_web_model_discovery_command;
pub(super) use config_web_no_arg_command;
pub(super) use config_web_update_command;
