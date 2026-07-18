use crate::actions::video_prompt_text::{
    MIN_FRAME_PROMPT_CHARS, MIN_PROVIDER_MOTION_PROMPT_CHARS, MIN_PROVIDER_VISUAL_PROMPT_CHARS,
    validate_provider_model_text,
};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_orchestrator::video::{PromptOutputLanguage, validate_direct_prompt_text};

pub(super) fn required_optimized_text(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("优化结果 {field} 不能为空"));
    }
    Ok(trimmed.to_string())
}

pub(super) fn normalized_optimized_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

pub(super) fn normalized_existing_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

pub(in crate::actions::video_prompts) fn optimized_motion_prompt(
    prompt: &chapter_video_prompt::Model,
    optimized: Option<String>,
) -> Result<Option<String>, String> {
    let optimized = normalized_optimized_optional_text(optimized);
    if prompt.prompt_type == "video_prompt" {
        return Ok(optimized
            .or_else(|| normalized_existing_optional_text(prompt.motion_prompt.as_deref())));
    }
    Ok(normalized_existing_optional_text(
        prompt.motion_prompt.as_deref(),
    ))
}

pub(in crate::actions::video_prompts) fn validate_optimized_prompt_fields(
    prompt: &chapter_video_prompt::Model,
    visual_prompt: &str,
    motion_prompt: Option<&str>,
    negative_prompt: Option<&str>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), String> {
    match prompt.prompt_type.as_str() {
        "video_prompt" => {
            validate_provider_model_text(
                visual_prompt,
                "visual_prompt",
                prompt.id,
                prompt_output_language,
                MIN_PROVIDER_VISUAL_PROMPT_CHARS,
                false,
            )?;
            let motion_prompt = motion_prompt.ok_or_else(|| {
                format!(
                    "优化结果 motion_prompt 不能为空；视频提示词 #{} 生成视频需要运镜与动作提示词",
                    prompt.id
                )
            })?;
            validate_provider_model_text(
                motion_prompt,
                "motion_prompt",
                prompt.id,
                prompt_output_language,
                MIN_PROVIDER_MOTION_PROMPT_CHARS,
                false,
            )?;
        }
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" => {
            validate_provider_model_text(
                visual_prompt,
                "frame_prompt",
                prompt.id,
                prompt_output_language,
                MIN_FRAME_PROMPT_CHARS,
                false,
            )?;
        }
        _ => {
            validate_direct_prompt_text(visual_prompt, "视频提示词 visual_prompt")
                .map_err(|err| err.to_string())?;
        }
    }
    if let Some(negative_prompt) = negative_prompt {
        validate_direct_prompt_text(negative_prompt, "视频提示词 negative_prompt")
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}
