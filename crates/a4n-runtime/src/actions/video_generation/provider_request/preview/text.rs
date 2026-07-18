use super::super::*;
use super::notes::append_provider_input_note;

pub(in crate::actions::video_generation) fn ensure_chapter_video_prompt_preview_texts(
    prompt: &mut chapter_video_prompt::Model,
    note: &mut Option<String>,
) {
    let prompt_output_language = PromptOutputLanguage::from_metadata(prompt.metadata_json.as_ref());
    prompt.visual_prompt = preview_model_text_or_placeholder(
        &prompt.visual_prompt,
        "visual_prompt",
        prompt_output_language,
        MIN_PROVIDER_VISUAL_PROMPT_CHARS,
        VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER,
        note,
    );
    let motion_prompt = prompt.motion_prompt.as_deref().unwrap_or_default();
    prompt.motion_prompt = Some(preview_model_text_or_placeholder(
        motion_prompt,
        "motion_prompt",
        prompt_output_language,
        MIN_PROVIDER_MOTION_PROMPT_CHARS,
        VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER,
        note,
    ));
}

fn preview_model_text_or_placeholder(
    value: &str,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
    min_chars: usize,
    placeholder: &str,
    note: &mut Option<String>,
) -> String {
    let trimmed = value.trim();
    let violation =
        provider_prompt_text_violation(trimmed, field, prompt_output_language, min_chars, false);
    if violation.is_none() {
        return trimmed.to_string();
    }
    match violation.expect("checked above") {
        ProviderPromptTextViolation::DictionaryLike => replace_with_preview_placeholder(
            note,
            field,
            "包含 JSON/字典结构片段",
            "真实生成前需要改写为自然语言",
            placeholder,
        ),
        ProviderPromptTextViolation::InternalWorkflowTerm(term) => {
            replace_with_preview_placeholder(
                note,
                field,
                &format!("包含视频生成内部流程说明“{term}”"),
                "真实生成前需要改写为只描述画面和动作的自然语言",
                placeholder,
            )
        }
        ProviderPromptTextViolation::UploadStatusTerm(term) => replace_with_preview_placeholder(
            note,
            field,
            &format!("包含上传状态说明“{term}”"),
            "真实生成前需要改写为实际起点、输入帧画面或上一段尾态",
            placeholder,
        ),
        ProviderPromptTextViolation::DuplicateMotionTimeline => replace_with_preview_placeholder(
            note,
            field,
            "重复镜头分镜",
            "真实生成前需要把时间轴只保留在 visual_prompt",
            placeholder,
        ),
        ProviderPromptTextViolation::MissingRequiredSection(section) => {
            replace_with_preview_placeholder(
                note,
                field,
                &format!("缺少必需章节 {section}"),
                "真实生成前需要补全完整提示词",
                placeholder,
            )
        }
        ProviderPromptTextViolation::StaleSection(section) => replace_with_preview_placeholder(
            note,
            field,
            &format!("包含已废弃章节 {section}"),
            "真实生成前需要重新生成当前格式提示词",
            placeholder,
        ),
        ProviderPromptTextViolation::DirectPrompt(message) => replace_with_preview_placeholder(
            note,
            field,
            &format!("不符合自然语言提示词约束：{message}"),
            "真实生成前需要改写",
            placeholder,
        ),
        ProviderPromptTextViolation::Empty => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 过短，至少需要 {min_chars} 个字符；预览使用占位文本展示请求结构，真实生成仍需补全。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::TooShort { .. } => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 过短，至少需要 {min_chars} 个字符；预览使用占位文本展示请求结构，真实生成仍需补全。"
                ),
            );
            format!("{trimmed}\n\n{placeholder}")
        }
        ProviderPromptTextViolation::PreviewPlaceholder => replace_with_preview_placeholder(
            note,
            field,
            "包含请求预览占位文本",
            "真实生成前需要保存完整提示词",
            placeholder,
        ),
    }
}

fn replace_with_preview_placeholder(
    note: &mut Option<String>,
    field: &str,
    violation: &str,
    requirement: &str,
    placeholder: &str,
) -> String {
    append_provider_input_note(
        note,
        format!("{field} {violation}；预览使用占位文本展示请求结构，{requirement}。"),
    );
    placeholder.to_string()
}
