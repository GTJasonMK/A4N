use serde_json::Value;

use super::shared::{
    config_web_id_unit_command, config_web_input_command, config_web_model_discovery_command,
    config_web_no_arg_command, config_web_update_command, i64_arg_or_system_default,
    require_global_config_admin,
};
use crate::routes::commands::{ApiError, ServerState, config_records_actions};

config_web_no_arg_command!(
    list_video_configs,
    config_records_actions::list_video_configs,
);
config_web_input_command!(
    add_video_config,
    config_records_actions::VideoConfigInput,
    config_records_actions::add_video_config,
);
config_web_update_command!(
    update_video_config,
    config_records_actions::UpdateVideoConfigInput,
    config_records_actions::update_video_config,
);
config_web_id_unit_command!(
    remove_video_config,
    config_records_actions::remove_video_config,
);
config_web_id_unit_command!(
    activate_video_config,
    config_records_actions::activate_video_config,
);

pub(in crate::routes::commands) async fn test_video_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg_or_system_default(args, "configId")?;
    if a4n_runtime::runtime_config::current_video_model_for_id(Some(config_id)).is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    config_records_actions::test_video_config(state.runtime(), config_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

config_web_model_discovery_command!(
    detect_video_config_models,
    config_records_actions::detect_video_config_models,
);
