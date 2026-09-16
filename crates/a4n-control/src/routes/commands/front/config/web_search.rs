use a4n_api_contract::web_search::WebSearchInput;
use serde_json::Value;

use crate::routes::commands::{
    ApiError, config_records_actions, strict_runtime_command_input, to_value,
};

pub(in crate::routes::commands) async fn web_search(args: &Value) -> Result<Value, ApiError> {
    let input: WebSearchInput = strict_runtime_command_input(args, "web_search")?;
    to_value(config_records_actions::web_search(input).await?)
}
