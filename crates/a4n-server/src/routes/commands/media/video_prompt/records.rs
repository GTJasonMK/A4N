use serde_json::Value;

use super::super::super::{
    ApiError, DownstreamConflictStrategy, ServerState, command_input, downstream_strategy_arg,
    i64_arg, nullable_i64_update_field, nullable_string_update_field, nullable_value_update_field,
    optional_i32_field, optional_string_field, required_arg, string_arg, to_value,
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
    let duration_secs = optional_i32_field(input, "duration_secs")?;
    let reference_image_ids_json = nullable_value_update_field(input, "reference_image_ids_json")?;
    let guidance_mode = optional_string_field(input, "guidance_mode")?;
    let continuity_source_video_id =
        nullable_i64_update_field(input, "continuity_source_video_id")?;
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
            duration_secs,
            reference_image_ids_json,
            guidance_mode,
            continuity_source_video_id,
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

pub(in crate::routes::commands) async fn delete_chapter_video_prompt_group(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let group = string_arg(args, "group")?;
    let downstream_strategy = downstream_strategy_arg(args)?;
    let result = video_prompt_actions::delete_chapter_video_prompt_group(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        &group,
        downstream_strategy,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(result)
}
