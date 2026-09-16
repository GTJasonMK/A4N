use a4n_api_contract::web_fetch::WebFetchInput;
use serde_json::Value;

use crate::routes::commands::{ApiError, config_records_actions, direct_command_input, to_value};

pub(in crate::routes::commands) async fn web_fetch(args: &Value) -> Result<Value, ApiError> {
    let input: WebFetchInput = direct_command_input(args, "web_fetch")?;
    to_value(config_records_actions::web_fetch(input).await?)
}
