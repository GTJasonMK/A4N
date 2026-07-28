//! 章节视频提示词优化结果纯校验（无 DB、无 Host）。

use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::prompt_language::PromptOutputLanguage;

use super::prompt_json::validate_direct_prompt_text;
use super::provider_prompt_text_rules::{
    MIN_FRAME_PROMPT_CHARS, MIN_PROVIDER_MOTION_PROMPT_CHARS, validate_provider_model_text,
};
use super::reference_media_rules::MIN_PROVIDER_VISUAL_PROMPT_CHARS;

/// 优化结果必填文本字段。
pub fn required_optimized_text(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("优化结果 {field} 不能为空"));
    }
    Ok(trimmed.to_string())
}

/// 优化结果可选字符串（trim 空 → None）。
pub fn normalized_optimized_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

/// 既有行可选字符串（trim 空 → None）。
pub fn normalized_existing_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

/// 合并优化输出与既有 motion_prompt（仅 video_prompt 类型允许优化覆盖）。
pub fn optimized_motion_prompt(
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

/// 校验优化后的 visual/motion/negative 文本是否可落库。
pub fn validate_optimized_prompt_fields(
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
