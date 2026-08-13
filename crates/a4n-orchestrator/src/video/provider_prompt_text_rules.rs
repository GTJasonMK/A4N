//! Provider 侧视频提示词文本纯规则（无 DB、无 Host IO）。

use super::prompt_json::{
    validate_direct_prompt_text, video_motion_prompt_has_temporal_storyboard,
    video_prompt_missing_required_section_for_language, video_prompt_stale_section_for_language,
    video_prompt_upload_status_term,
};
use a4n_core::prompt_language::PromptOutputLanguage;

/// 与 motion prompt 下限一致（单一来源）。
pub const MIN_PROVIDER_MOTION_PROMPT_CHARS: usize = 60;
/// 帧提示词下限。
pub const MIN_FRAME_PROMPT_CHARS: usize = 80;

/// 视频提示词文本违规分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderPromptTextViolation {
    Empty,
    TooShort {
        min_chars: usize,
        actual_chars: usize,
    },
    DictionaryLike,
    PreviewPlaceholder,
    InternalWorkflowTerm(&'static str),
    UploadStatusTerm(&'static str),
    StaleSection(&'static str),
    DuplicateMotionTimeline,
    MissingRequiredSection(&'static str),
    DirectPrompt(String),
}

const INTERNAL_VIDEO_PROMPT_TERMS: &[&str] = &[
    "首帧串联",
    "串联规则",
    "自动采集",
    "系统采集",
    "供系统",
    "上一段视频",
    "上一段尾帧",
    "下一段首帧",
    "provider",
    "供应商请求",
    "image 字段",
    "first_frame_image_url",
    "generated_image #",
];

/// 校验 provider 模型文本；失败时带 prompt_id 文案。
pub fn validate_provider_model_text<'a>(
    value: &'a str,
    field: &str,
    prompt_id: i64,
    prompt_output_language: PromptOutputLanguage,
    min_chars: usize,
    allow_preview_placeholders: bool,
) -> Result<&'a str, String> {
    validate_provider_model_text_for_context(
        value,
        field,
        &format!("视频提示词 #{prompt_id}"),
        prompt_output_language,
        min_chars,
        allow_preview_placeholders,
    )
}

pub(in crate::video) fn validate_provider_model_text_for_context<'a>(
    value: &'a str,
    field: &str,
    context: &str,
    prompt_output_language: PromptOutputLanguage,
    min_chars: usize,
    allow_preview_placeholders: bool,
) -> Result<&'a str, String> {
    let text = value.trim();
    if let Some(violation) = provider_prompt_text_violation(
        text,
        field,
        prompt_output_language,
        min_chars,
        allow_preview_placeholders,
    ) {
        return Err(provider_prompt_text_error(context, field, violation));
    }
    Ok(text)
}

/// 检测 provider 提示词违规（不含 prompt_id 文案包装）。
pub fn provider_prompt_text_violation(
    text: &str,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
    min_chars: usize,
    allow_preview_placeholders: bool,
) -> Option<ProviderPromptTextViolation> {
    let text = text.trim();
    if text.is_empty() {
        return Some(ProviderPromptTextViolation::Empty);
    }
    let char_count = text.chars().count();
    if contains_dictionary_like_prompt_text(text) {
        return Some(ProviderPromptTextViolation::DictionaryLike);
    }
    if text.contains("请求预览占位") && !allow_preview_placeholders {
        return Some(ProviderPromptTextViolation::PreviewPlaceholder);
    }
    if let Some(term) = internal_video_workflow_term(text) {
        return Some(ProviderPromptTextViolation::InternalWorkflowTerm(term));
    }
    if let Some(term) = video_prompt_upload_status_term(text) {
        return Some(ProviderPromptTextViolation::UploadStatusTerm(term));
    }
    if let Some(section) = video_prompt_stale_section_for_language(field, text) {
        return Some(ProviderPromptTextViolation::StaleSection(section));
    }
    if field == "motion_prompt" && video_motion_prompt_has_temporal_storyboard(text) {
        return Some(ProviderPromptTextViolation::DuplicateMotionTimeline);
    }
    if char_count < min_chars {
        return Some(ProviderPromptTextViolation::TooShort {
            min_chars,
            actual_chars: char_count,
        });
    }
    if let Some(section) =
        video_prompt_missing_required_section_for_language(field, text, prompt_output_language)
    {
        return Some(ProviderPromptTextViolation::MissingRequiredSection(section));
    }
    if let Err(err) = validate_direct_prompt_text(text, field) {
        return Some(ProviderPromptTextViolation::DirectPrompt(err.to_string()));
    }
    None
}

/// 将违规转为带 prompt_id 的错误文案。
fn provider_prompt_text_error(
    context: &str,
    field: &str,
    violation: ProviderPromptTextViolation,
) -> String {
    match violation {
        ProviderPromptTextViolation::Empty => format!("{context} 缺少 {field}"),
        ProviderPromptTextViolation::TooShort {
            min_chars,
            actual_chars,
        } => format!(
            "{context} 的 {field} 过短，至少需要 {min_chars} 个字符，当前 {actual_chars} 个字符；请重新生成或补全视频提示词"
        ),
        ProviderPromptTextViolation::DictionaryLike => format!(
            "{context} 的 {field} 包含 JSON/字典结构片段，请重新生成或改写为自然语言提示词后再生成视频"
        ),
        ProviderPromptTextViolation::PreviewPlaceholder => {
            format!("{context} 的 {field} 包含请求预览占位文本，请保存完整提示词后再生成视频")
        }
        ProviderPromptTextViolation::InternalWorkflowTerm(term) => format!(
            "{context} 的 {field} 包含视频生成内部流程说明“{term}”，请重新生成或改写为只描述画面和动作的自然语言提示词后再生成视频"
        ),
        ProviderPromptTextViolation::UploadStatusTerm(term) => format!(
            "{context} 的 {field} 包含上传状态说明“{term}”，请优化或重新生成为只描述实际起点、输入帧画面或上一段尾态的自然语言提示词后再生成视频"
        ),
        ProviderPromptTextViolation::StaleSection(section) => format!(
            "{context} 的 {field} 使用旧版视频提示词章节 {section}，请重新生成或优化为当前格式后再生成视频"
        ),
        ProviderPromptTextViolation::DuplicateMotionTimeline => format!(
            "{context} 的 motion_prompt 不应重复镜头分镜；请把镜头分镜、连续性和尾态只保留在 visual_prompt 中，motion_prompt 只写运镜、动作和表演补充"
        ),
        ProviderPromptTextViolation::MissingRequiredSection(section) => {
            format!("{context} 的 {field} 缺少必需章节 {section}，请重新生成或补全视频提示词")
        }
        ProviderPromptTextViolation::DirectPrompt(message) => message,
    }
}

fn contains_dictionary_like_prompt_text(text: &str) -> bool {
    text.contains('{')
        || text.contains('}')
        || text.contains("[{")
        || text.contains("}]")
        || text.contains("\":")
        || text.contains("':")
}

fn internal_video_workflow_term(text: &str) -> Option<&'static str> {
    let normalized = text.to_ascii_lowercase();
    INTERNAL_VIDEO_PROMPT_TERMS
        .iter()
        .copied()
        .find(|term| normalized.contains(term))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::MIN_PROVIDER_VISUAL_PROMPT_CHARS;

    #[test]
    fn hard_prompt_text_violations_take_priority_over_length() {
        let dictionary_violation = provider_prompt_text_violation(
            r#"{"visual_prompt":"test"}"#,
            "visual_prompt",
            PromptOutputLanguage::ZhCn,
            MIN_PROVIDER_VISUAL_PROMPT_CHARS,
            false,
        );
        assert_eq!(
            dictionary_violation,
            Some(ProviderPromptTextViolation::DictionaryLike)
        );

        let upload_status_violation = provider_prompt_text_violation(
            "没有输入首帧",
            "visual_prompt",
            PromptOutputLanguage::ZhCn,
            MIN_PROVIDER_VISUAL_PROMPT_CHARS,
            false,
        );
        assert_eq!(
            upload_status_violation,
            Some(ProviderPromptTextViolation::UploadStatusTerm(
                "没有输入首帧"
            ))
        );

        let short_natural_violation = provider_prompt_text_violation(
            "镜头缓慢推进",
            "visual_prompt",
            PromptOutputLanguage::ZhCn,
            MIN_PROVIDER_VISUAL_PROMPT_CHARS,
            false,
        );
        assert!(matches!(
            short_natural_violation,
            Some(ProviderPromptTextViolation::TooShort { .. })
        ));
    }
}
