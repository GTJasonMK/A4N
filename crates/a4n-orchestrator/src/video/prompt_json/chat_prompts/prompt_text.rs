use a4n_core::error::AppError;
use a4n_core::prompt_language::PromptOutputLanguage;

use crate::video::prompt_json::validation::{
    validate_direct_prompt_text, validate_prompt_has_no_upload_status,
    validate_prompt_has_shot_storyboard_section,
};

pub(super) fn required_text(value: String, field: &str) -> Result<String, AppError> {
    let text = value.trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "Chat 视频提示词 {field} 不能为空"
        )));
    }
    Ok(text.to_string())
}

pub(in crate::video) fn validate_chat_video_prompt_text(
    text: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    validate_direct_prompt_text(text, "Chat 视频提示词 chat_prompt")?;
    validate_prompt_has_no_upload_status(text, "Chat 视频提示词 chat_prompt")?;
    validate_chat_prompt_required_sections(text, prompt_output_language)?;
    validate_prompt_has_shot_storyboard_section(
        text,
        "Chat 视频提示词 chat_prompt",
        prompt_output_language,
    )?;
    let avoidance_section = prompt_output_language
        .visual_sections()
        .last()
        .and_then(|section| chat_prompt_section(text, section))
        .unwrap_or("");
    if !contains_no_background_music_rule(avoidance_section) {
        return Err(AppError::InvalidArgument(
            "Chat 视频提示词 chat_prompt 的限制与避免项必须明确包含不要配背景音乐/配乐/BGM/background music 的限制"
                .to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_chat_prompt_required_sections(
    text: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    for section in prompt_output_language.visual_sections() {
        if !text.contains(section) {
            return Err(AppError::InvalidArgument(format!(
                "Chat 视频提示词 chat_prompt 缺少章节 {section}"
            )));
        }
    }
    Ok(())
}

pub(super) fn chat_prompt_section<'a>(text: &'a str, section: &str) -> Option<&'a str> {
    let start = text.find(section)?;
    let content_start = start + section.len();
    let rest = &text[content_start..];
    let end = rest.find("\n[").unwrap_or(rest.len());
    Some(rest[..end].trim())
}

pub(super) fn contains_no_background_music_rule(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    let no_music_phrases = [
        "不要配背景音乐",
        "不要添加背景音乐",
        "不要加背景音乐",
        "禁止背景音乐",
        "无背景音乐",
        "不要背景音乐",
        "不要配乐",
        "禁止配乐",
        "无配乐",
        "不要加配乐",
        "no background music",
        "without background music",
        "do not add background music",
        "do not include background music",
        "no soundtrack",
        "without soundtrack",
        "no bgm",
    ];
    no_music_phrases
        .iter()
        .any(|phrase| text.contains(phrase) || lowercase.contains(phrase))
        || ((lowercase.contains("bgm") || lowercase.contains("background music"))
            && [
                "不要",
                "禁止",
                "无",
                "不要加",
                "不要添加",
                "不要配",
                "no",
                "without",
                "do not",
                "don't",
            ]
            .iter()
            .any(|prefix| text.contains(prefix) || lowercase.contains(prefix)))
}
