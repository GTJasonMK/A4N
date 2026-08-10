use serde_json::Value;

use crate::routes::commands::{
    ApiError, ServerState, command_input, config_records_actions, to_value,
};

pub(in crate::routes::commands) async fn get_usage_guide(
    state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    to_value(config_records_actions::get_usage_guide(state.runtime()).await?)
}

pub(in crate::routes::commands) async fn update_usage_guide(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: config_records_actions::UsageGuideDocument =
        command_input(args, "update_usage_guide")?;
    to_value(config_records_actions::update_usage_guide(state.runtime(), user_id, input).await?)
}
