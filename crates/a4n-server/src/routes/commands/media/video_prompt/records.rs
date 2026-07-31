use serde_json::Value;

use super::super::super::{
    ApiError, DownstreamConflictStrategy, ServerState, command_input, downstream_strategy_arg,
    i64_arg, nullable_string_update_field, optional_string_field, required_arg, to_value,
    video_prompt_actions,
};

pub(in crate::routes::commands) async fn list_chapter_video_prompts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let prompts = video_prompt_actions::list_chapter_video_prompts(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(prompts)
}

pub(in crate::routes::commands) async fn load_video_segment_plan_draft(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: video_prompt_actions::LoadVideoSegmentPlanDraftInput =
        command_input(args, "load_video_segment_plan_draft")?;
    let detail =
        video_prompt_actions::load_video_segment_plan_draft(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(detail)
}

pub(in crate::routes::commands) async fn update_video_segment_plan_draft_recipe(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: video_prompt_actions::UpdateVideoSegmentPlanDraftRecipeInput =
        command_input(args, "update_video_segment_plan_draft_recipe")?;
    let detail = video_prompt_actions::update_video_segment_plan_draft_recipe(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(detail)
}

pub(in crate::routes::commands) async fn update_chapter_video_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let input = required_arg(args, "input")?;
    let visual_prompt = optional_string_field(input, "visual_prompt")?;
    let motion_prompt = nullable_string_update_field(input, "motion_prompt")?;
    let negative_prompt = nullable_string_update_field(input, "negative_prompt")?;
    let segment_recipe = input
        .get("segment_recipe")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| ApiError::bad_request(format!("segment_recipe 无效：{error}")))?;
    let downstream_strategy = DownstreamConflictStrategy::from_optional_str(
        optional_string_field(input, "downstream_strategy")?.as_deref(),
    )
    .map_err(ApiError::bad_request)?;
    let updated = video_prompt_actions::update_chapter_video_prompt(
        state.runtime(),
        user_id,
        prompt_id,
        video_prompt_actions::UpdateChapterVideoPromptInput {
            visual_prompt,
            motion_prompt,
            negative_prompt,
            segment_recipe,
            downstream_strategy,
        },
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn optimize_chapter_video_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let input: video_prompt_actions::OptimizeChapterVideoPromptInput =
        command_input(args, "optimize_chapter_video_prompt")?;
    let updated = video_prompt_actions::optimize_chapter_video_prompt(
        state.runtime(),
        user_id,
        prompt_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn delete_chapter_video_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let downstream_strategy = downstream_strategy_arg(args)?;
    video_prompt_actions::delete_chapter_video_prompt(
        state.runtime(),
        user_id,
        prompt_id,
        downstream_strategy,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}
