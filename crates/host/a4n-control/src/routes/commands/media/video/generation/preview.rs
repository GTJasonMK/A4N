use serde_json::Value;

use super::super::super::super::{
    ApiError, ServerState, i64_arg, to_value, video_generation_actions,
};
use super::input::{
    chapter_video_generation_input, optional_prompt_override,
    reference_media_video_generation_input,
};

pub(in crate::routes::commands) async fn preview_chapter_video_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = chapter_video_generation_input(user_id, args)?;
    let prompt_override = optional_prompt_override(args, "preview_chapter_video_request")?;
    let preview = video_generation_actions::preview_chapter_video_request(
        state.runtime(),
        input,
        prompt_override,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn preview_video_provider_request_by_video_id(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let video_id = i64_arg(args, "videoId")?;
    let preview = video_generation_actions::preview_video_provider_request_by_video_id(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        video_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn preview_reference_media_video_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = reference_media_video_generation_input(
        user_id,
        args,
        "preview_reference_media_video_request",
    )?;
    let preview =
        video_generation_actions::preview_reference_media_video_request(state.runtime(), input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}
