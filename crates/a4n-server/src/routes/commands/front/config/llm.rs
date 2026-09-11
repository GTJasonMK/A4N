use a4n_api_contract::config::{CreateLlmInput, LlmMaxTokensConfig, UpdateLlmInput};
use serde_json::Value;

use super::shared::{
    config_web_id_unit_command, config_web_input_command, config_web_model_discovery_command,
    config_web_no_arg_command, config_web_update_command, i64_arg_or_system_default,
};
use crate::routes::commands::{ApiError, ServerState, config_records_actions, to_value};

config_web_no_arg_command!(list_llm_config, config_records_actions::list_llm_config);
config_web_input_command!(
    create_llm_config,
    CreateLlmInput,
    config_records_actions::create_llm_config,
);
config_web_update_command!(
    update_llm_config,
    UpdateLlmInput,
    config_records_actions::update_llm_config,
);
config_web_id_unit_command!(
    set_default_llm_config,
    config_records_actions::set_default_llm_config,
);
config_web_id_unit_command!(delete_llm_config, config_records_actions::delete_llm_config);
config_web_no_arg_command!(
    get_llm_max_tokens_config,
    config_records_actions::get_llm_max_tokens_config,
);
config_web_input_command!(
    update_llm_max_tokens_config,
    LlmMaxTokensConfig,
    config_records_actions::update_llm_max_tokens_config,
);

pub(in crate::routes::commands) async fn test_llm_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    crate::routes::commands::reject_unknown_command_args(
        args,
        "test_llm_config",
        &["configId", "runtime_config_snapshot"],
    )?;
    let config_id = i64_arg_or_system_default(args, "configId")?;
    to_value(
        config_records_actions::test_llm_config(state.runtime(), user_id, config_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn preview_test_llm_config_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    crate::routes::commands::reject_unknown_command_args(
        args,
        "preview_test_llm_config_request",
        &["configId", "runtime_config_snapshot"],
    )?;
    let config_id = i64_arg_or_system_default(args, "configId")?;
    to_value(
        config_records_actions::preview_test_llm_config_request(
            state.runtime(),
            user_id,
            config_id,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

config_web_model_discovery_command!(
    detect_llm_config_models,
    config_records_actions::detect_llm_config_models,
);
