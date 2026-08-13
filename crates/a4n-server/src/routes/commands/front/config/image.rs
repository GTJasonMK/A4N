use serde_json::Value;

use super::shared::{
    config_web_id_unit_command, config_web_input_command, config_web_model_discovery_command,
    config_web_no_arg_command, config_web_update_command, i64_arg_or_system_default,
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
    set_default_image_config,
    config_records_actions::set_default_image_config,
);

pub(in crate::routes::commands) async fn test_image_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    crate::routes::commands::reject_unknown_command_args(
        args,
        "test_image_config",
        &["configId", "runtime_config_snapshot"],
    )?;
    let config_id = i64_arg_or_system_default(args, "configId")?;
    config_records_actions::test_image_config(state.runtime(), user_id, config_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn preview_test_image_config_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    crate::routes::commands::reject_unknown_command_args(
        args,
        "preview_test_image_config_request",
        &["configId", "runtime_config_snapshot"],
    )?;
    let config_id = i64_arg_or_system_default(args, "configId")?;
    to_value(
        config_records_actions::preview_test_image_config_request(
            state.runtime(),
            user_id,
            config_id,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

config_web_model_discovery_command!(
    detect_image_config_models,
    config_records_actions::detect_image_config_models,
);
