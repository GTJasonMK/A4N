use a4n_api_contract::web_search::UpdateWebSearchConfigInput;
use a4n_api_contract::web_search::WebSearchInput;
use serde_json::Value;

use crate::routes::commands::{
    ApiError, ServerState, config_records_actions, empty_command_args, strict_command_input,
    to_value,
};

pub(in crate::routes::commands) async fn get_web_search_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "get_web_search_config")?;
    to_value(
        config_records_actions::get_web_search_config(state.runtime(), user_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn update_web_search_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: UpdateWebSearchConfigInput = strict_command_input(args, "update_web_search_config")?;
    to_value(
        config_records_actions::update_web_search_config(state.runtime(), user_id, input).await?,
    )
}

pub(in crate::routes::commands) async fn web_search(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: WebSearchInput = strict_command_input(args, "web_search")?;
    to_value(config_records_actions::web_search(state.runtime(), input).await?)
}
