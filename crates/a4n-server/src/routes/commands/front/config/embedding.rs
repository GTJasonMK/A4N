use serde_json::{Value, json};

use super::shared::{
    config_web_id_unit_command, config_web_input_command, config_web_no_arg_command,
    config_web_update_command, require_global_config_admin,
};
use crate::routes::commands::{ApiError, ServerState, config_records_actions, i64_arg, to_value};

config_web_no_arg_command!(
    list_embedding_config,
    config_records_actions::list_embedding_config,
);
config_web_input_command!(
    create_embedding_config,
    config_records_actions::CreateEmbeddingInput,
    config_records_actions::create_embedding_config,
);
config_web_update_command!(
    update_embedding_config,
    config_records_actions::UpdateEmbeddingInput,
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
    let config_id = i64_arg(args, "configId")?;
    if a4n_runtime::runtime_config::current_embedding_model().is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    config_records_actions::test_embedding_config(state.runtime(), config_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn preview_test_embedding_config_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg(args, "configId")?;
    if a4n_runtime::runtime_config::current_embedding_model().is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    to_value(
        config_records_actions::preview_test_embedding_config_request(state.runtime(), config_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn list_model_provider_catalog(
    _state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    Ok(json!(config_records_actions::model_provider_catalog()))
}
