use serde_json::Value;

use super::super::super::{
    ApiError, command_input, i64_arg, nullable_i64_arg, video_generation_actions,
};

pub(super) fn chapter_video_generation_input(
    user_id: i64,
    args: &Value,
) -> Result<video_generation_actions::GenerateChapterVideoInput, ApiError> {
    Ok(video_generation_actions::GenerateChapterVideoInput {
        user_id,
        project_id: i64_arg(args, "projectId")?,
        chapter_id: i64_arg(args, "chapterId")?,
        prompt_id: i64_arg(args, "promptId")?,
        config_id: nullable_i64_arg(args, "configId")?,
    })
}

pub(super) fn reference_media_video_generation_input(
    user_id: i64,
    args: &Value,
    command: &str,
) -> Result<video_generation_actions::GenerateReferenceMediaVideoInput, ApiError> {
    let input = command_input(args, command)?;
    Ok(video_generation_actions::GenerateReferenceMediaVideoInput {
        user_id,
        project_id: i64_arg(args, "projectId")?,
        chapter_id: i64_arg(args, "chapterId")?,
        input,
    })
}

pub(super) fn optional_prompt_override(
    args: &Value,
    command: &str,
) -> Result<Option<video_generation_actions::PreviewChapterVideoPromptOverrideInput>, ApiError> {
    match args.get("promptOverride") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|error| {
                ApiError::bad_request(format!("{command}.promptOverride 无效: {error}"))
            }),
    }
}
