use a4n_api_contract::config::{CreateEmbeddingInput, UpdateEmbeddingInput};
use serde_json::Value;

use super::shared::{
    config_web_id_unit_command, config_web_input_command, config_web_no_arg_command,
    config_web_update_command,
};
use crate::routes::commands::{ApiError, ServerState, config_records_actions, i64_arg, to_value};

config_web_no_arg_command!(
    list_embedding_config,
    config_records_actions::list_embedding_config,
);
config_web_input_command!(
    create_embedding_config,
    CreateEmbeddingInput,
    config_records_actions::create_embedding_config,
);
config_web_update_command!(
    update_embedding_config,
    UpdateEmbeddingInput,
    config_records_actions::update_embedding_config,
);
config_web_id_unit_command!(
    set_default_embedding_config,
    config_records_actions::set_default_embedding_config,
);
config_web_id_unit_command!(
    delete_embedding_config,
    config_records_actions::delete_embedding_config,
);
pub(in crate::routes::commands) async fn test_embedding_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    crate::routes::commands::reject_unknown_command_args(
        args,
        "test_embedding_config",
        &["configId", "runtime_config_snapshot"],
    )?;
    let config_id = i64_arg(args, "configId")?;
    config_records_actions::test_embedding_config(state.runtime(), user_id, config_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn preview_test_embedding_config_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    crate::routes::commands::reject_unknown_command_args(
        args,
        "preview_test_embedding_config_request",
        &["configId", "runtime_config_snapshot"],
    )?;
    let config_id = i64_arg(args, "configId")?;
    to_value(
        config_records_actions::preview_test_embedding_config_request(
            state.runtime(),
            user_id,
            config_id,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn list_model_provider_catalog(
    _state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    crate::routes::commands::empty_command_args(args, "list_model_provider_catalog")?;
    to_value(config_records_actions::model_provider_catalog())
}
