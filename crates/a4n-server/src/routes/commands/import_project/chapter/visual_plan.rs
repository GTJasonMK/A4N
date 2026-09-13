use a4n_runtime::host_api::chapter_visual_plan_records as visual_plan_actions;
use serde_json::{Value, json};

use crate::routes::commands::{ApiError, strict_command_input, string_action_result};
use crate::state::ServerState;

pub(in crate::routes::commands) async fn preview_visual_plan_point_generation(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "preview_visual_plan_point_generation")?;
    Ok(json!(
        visual_plan_actions::preview_visual_plan_point_generation(state.runtime(), user_id, input)
            .await?
    ))
}

pub(in crate::routes::commands) async fn read_visual_plan_prompt_candidate(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "read_visual_plan_prompt_candidate")?;
    Ok(json!(
        visual_plan_actions::read_visual_plan_prompt_candidate(state.runtime(), user_id, input)
            .await?
    ))
}

pub(in crate::routes::commands) async fn read_chapter_visual_plan(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "read_chapter_visual_plan")?;
    Ok(json!(
        visual_plan_actions::read_chapter_visual_plan(state.runtime(), user_id, input).await?
    ))
}

pub(in crate::routes::commands) async fn edit_chapter_visual_plan_draft(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "edit_chapter_visual_plan_draft")?;
    Ok(json!(
        visual_plan_actions::edit_chapter_visual_plan_draft(state.runtime(), user_id, input)
            .await?
    ))
}

pub(in crate::routes::commands) async fn validate_chapter_visual_plan(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "validate_chapter_visual_plan")?;
    Ok(json!(
        visual_plan_actions::validate_chapter_visual_plan(state.runtime(), user_id, input).await?
    ))
}

pub(in crate::routes::commands) async fn get_chapter_visual_plan(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "get_chapter_visual_plan")?;
    Ok(json!(string_action_result(
        visual_plan_actions::get_chapter_visual_plan(state.runtime(), user_id, input).await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn save_chapter_visual_plan_draft(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "save_chapter_visual_plan_draft")?;
    Ok(json!(string_action_result(
        visual_plan_actions::save_chapter_visual_plan_draft(state.runtime(), user_id, input).await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn confirm_chapter_visual_plan(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "confirm_chapter_visual_plan")?;
    Ok(json!(string_action_result(
        visual_plan_actions::confirm_chapter_visual_plan(state.runtime(), user_id, input).await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn discard_chapter_visual_plan_draft(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "discard_chapter_visual_plan_draft")?;
    Ok(json!(string_action_result(
        visual_plan_actions::discard_chapter_visual_plan_draft(state.runtime(), user_id, input)
            .await,
        false,
    )?))
}
