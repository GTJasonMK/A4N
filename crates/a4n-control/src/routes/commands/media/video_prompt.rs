use serde_json::Value;

use super::super::{
    ApiError, ServerState, command_input, i64_arg, required_arg, to_value, video_prompt_actions,
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

pub(in crate::routes::commands) async fn get_chapter_video_timeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: video_prompt_actions::GetVideoTimelineInput =
        command_input(args, "get_chapter_video_timeline")?;
    let timeline =
        video_prompt_actions::get_chapter_video_timeline(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(timeline)
}

pub(in crate::routes::commands) async fn commit_inserted_segment_draft(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: video_prompt_actions::CommitInsertedSegmentDraftInput =
        command_input(args, "commit_inserted_segment_draft")?;
    let timeline =
        video_prompt_actions::commit_inserted_segment_draft(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(timeline)
}

pub(in crate::routes::commands) async fn move_inserted_segment(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: video_prompt_actions::MoveInsertedSegmentInput =
        command_input(args, "move_inserted_segment")?;
    let timeline = video_prompt_actions::move_inserted_segment(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(timeline)
}

pub(in crate::routes::commands) async fn delete_inserted_segment(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: video_prompt_actions::DeleteInsertedSegmentInput =
        command_input(args, "delete_inserted_segment")?;
    let timeline = video_prompt_actions::delete_inserted_segment(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(timeline)
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
    video_prompt_actions::update_video_segment_plan_draft_recipe(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
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

pub(in crate::routes::commands) async fn update_chapter_video_segment_plan(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let input: video_prompt_actions::UpdateChapterVideoSegmentPlanInput =
        command_input(args, "update_chapter_video_segment_plan")?;
    let updated = video_prompt_actions::update_chapter_video_segment_plan(
        state.runtime(),
        user_id,
        prompt_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(updated)
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
    let input: video_prompt_actions::DeleteChapterVideoPromptInput =
        command_input(args, "delete_chapter_video_prompt")?;
    video_prompt_actions::delete_chapter_video_prompt(
        state.runtime(),
        user_id,
        input.prompt_id,
        input.downstream_strategy,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}
