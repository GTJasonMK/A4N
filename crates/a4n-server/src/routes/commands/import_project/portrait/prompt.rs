use crate::state::ServerState;
use serde_json::{Value, json};

use super::super::super::{ApiError, command_input, portrait_generation_actions, to_value};

pub(in crate::routes::commands) async fn generate_portrait_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: portrait_generation_actions::GeneratePortraitPromptInput =
        command_input(args, "generate_portrait_prompt")?;
    let prompt =
        portrait_generation_actions::generate_portrait_prompt(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    Ok(json!(prompt))
}

pub(in crate::routes::commands) async fn preview_generate_portrait_prompt_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: portrait_generation_actions::GeneratePortraitPromptInput =
        command_input(args, "preview_generate_portrait_prompt_request")?;
    let preview = portrait_generation_actions::preview_generate_portrait_prompt_request(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}
