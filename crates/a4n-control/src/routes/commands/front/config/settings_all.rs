use a4n_api_contract::config::ImportAllConfigsInput;
use serde_json::{Value, json};

use crate::routes::commands::{ApiError, ServerState, command_input};

pub(in crate::routes::commands) async fn export_all_configs_json(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    Ok(json!(
        a4n_runtime::host_api::settings_all::export_all_configs_to_json(state.runtime(), user_id)
            .await?
    ))
}

pub(in crate::routes::commands) async fn import_all_configs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: ImportAllConfigsInput = command_input(args, "import_all_configs")?;
    Ok(json!(
        a4n_runtime::host_api::settings_all::import_all_configs(state.runtime(), user_id, input)
            .await?
    ))
}
