use serde_json::Value;

use super::super::super::{ApiError, required_arg};

const SYSTEM_DEFAULT_CONFIG_ID: i64 = -1;

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
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            $crate::routes::commands::empty_command_args(args, stringify!($name))?;
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
            let input: $input =
                $crate::routes::commands::strict_command_input(args, stringify!($name))?;
            $crate::routes::commands::to_value($action(state.runtime(), user_id, input).await?)
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
            $crate::routes::commands::reject_unknown_command_args(
                args,
                stringify!($name),
                &["configId", "input"],
            )?;
            let config_id = $crate::routes::commands::i64_arg(args, "configId")?;
            let input: $input = $crate::routes::commands::command_input(args, stringify!($name))?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), user_id, config_id, input).await?,
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
            $crate::routes::commands::reject_unknown_command_args(
                args,
                stringify!($name),
                &["configId"],
            )?;
            let config_id = $crate::routes::commands::i64_arg(args, "configId")?;
            $action(state.runtime(), user_id, config_id).await?;
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
            $crate::routes::commands::reject_unknown_command_args(
                args,
                stringify!($name),
                &["input"],
            )?;
            let input: a4n_api_contract::config::ModelDiscoveryInput =
                $crate::routes::commands::command_input(args, stringify!($name))?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), user_id, input)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

pub(super) use config_web_id_unit_command;
pub(super) use config_web_input_command;
pub(super) use config_web_model_discovery_command;
pub(super) use config_web_no_arg_command;
pub(super) use config_web_update_command;
