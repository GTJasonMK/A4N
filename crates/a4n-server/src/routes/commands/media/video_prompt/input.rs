use serde_json::Value;

use super::super::super::{
    ApiError, i64_arg, optional_bool_field, optional_string_field, required_arg, required_json_i32,
    string_arg, video_prompt_actions,
};
use super::rules::{json_i64_vec, optional_json_i32_vec, optional_json_i64_vec};

pub(super) fn video_prompt_input_from_args(
    user_id: i64,
    args: &Value,
    command: &str,
) -> Result<video_prompt_actions::GenerateChapterVideoPromptInput, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let options = required_arg(args, "options")?;
    Ok(video_prompt_actions::GenerateChapterVideoPromptInput {
        user_id,
        project_id,
        chapter_id,
        options: video_prompt_actions::GeneratePromptOptionsInput {
            workflow_mode: optional_string_field(options, "workflow_mode")?,
            style: string_arg(options, "style")?,
            prompt_output_language: optional_string_field(options, "prompt_output_language")?,
            prompt_batch_size: required_json_i32(options, "prompt_batch_size", command)?,
            duration_secs: required_json_i32(options, "duration_secs", command)?,
            duration_secs_options: optional_json_i32_vec(
                options,
                "duration_secs_options",
                command,
            )?,
            aspect_ratio: string_arg(options, "aspect_ratio")?,
            reference_image_ids: json_i64_vec(options, "reference_image_ids", command)?,
            video_library_image_ids: optional_json_i64_vec(
                options,
                "video_library_image_ids",
                command,
            )?,
            regenerate_existing: optional_bool_field(options, "regenerate_existing")?,
        },
    })
}
