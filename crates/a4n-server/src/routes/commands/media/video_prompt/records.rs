use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, command_input, downstream_strategy_arg, i64_arg, required_arg, to_value,
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
    let result = video_prompt_actions::update_video_segment_plan_draft_recipe(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(in crate::routes::commands) async fn update_chapter_video_segment_recipe(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let recipe = required_arg(args, "recipe")?;
    let recipe = serde_json::from_value(recipe.clone())
        .map_err(|error| ApiError::bad_request(format!("recipe 无效：{error}")))?;
    let recipes = video_prompt_actions::update_chapter_video_segment_recipe(
        state.runtime(),
        user_id,
        prompt_id,
        recipe,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(recipes)
}

pub(in crate::routes::commands) async fn update_chapter_video_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let input: video_prompt_actions::UpdateChapterVideoPromptInput =
        command_input(args, "update_chapter_video_prompt")?;
    let updated = video_prompt_actions::update_chapter_video_prompt(
        state.runtime(),
        user_id,
        prompt_id,
        input,
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
