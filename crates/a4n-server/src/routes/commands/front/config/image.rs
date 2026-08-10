use serde_json::Value;

use super::shared::{
    config_web_id_unit_command, config_web_input_command, config_web_model_discovery_command,
    config_web_no_arg_command, config_web_update_command, i64_arg_or_system_default,
    require_global_config_admin,
};
use crate::routes::commands::{ApiError, ServerState, config_records_actions, to_value};

config_web_no_arg_command!(
    list_image_configs,
    config_records_actions::list_image_configs,
);
config_web_input_command!(
    add_image_config,
    config_records_actions::ImageConfigInput,
    config_records_actions::add_image_config,
);
config_web_update_command!(
    update_image_config,
    config_records_actions::UpdateImageConfigInput,
    config_records_actions::update_image_config,
);
config_web_id_unit_command!(
    remove_image_config,
    config_records_actions::remove_image_config,
);
config_web_id_unit_command!(
    activate_image_config,
    config_records_actions::activate_image_config,
);

pub(in crate::routes::commands) async fn test_image_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg_or_system_default(args, "configId")?;
    if a4n_runtime::runtime_config::current_image_model_for_id(Some(config_id)).is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    config_records_actions::test_image_config(state.runtime(), config_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn preview_test_image_config_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg_or_system_default(args, "configId")?;
    if a4n_runtime::runtime_config::current_image_model_for_id(Some(config_id)).is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    to_value(
        config_records_actions::preview_test_image_config_request(state.runtime(), config_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

config_web_model_discovery_command!(
    detect_image_config_models,
    config_records_actions::detect_image_config_models,
);
