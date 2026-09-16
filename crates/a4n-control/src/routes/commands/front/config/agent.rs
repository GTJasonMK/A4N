use a4n_api_contract::agent::UpdateAgentSkillConfigurationInput;
use serde_json::Value;

use crate::routes::commands::{
    ApiError, ServerState, empty_command_args, strict_command_input, to_value,
};

pub(in crate::routes::commands) async fn list_agent_skills(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "list_agent_skills")?;
    to_value(
        a4n_runtime::host_api::agent_runtime::list_skill_settings(state.runtime(), user_id).await?,
    )
}

pub(in crate::routes::commands) async fn update_agent_skill_configuration(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: UpdateAgentSkillConfigurationInput =
        strict_command_input(args, "update_agent_skill_configuration")?;
    to_value(
        a4n_runtime::host_api::agent_runtime::update_skill_configuration(
            state.runtime(),
            user_id,
            input,
        )
        .await?,
    )
}
