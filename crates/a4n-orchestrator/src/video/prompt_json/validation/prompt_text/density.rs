use super::references::validate_direct_prompt_text;
use crate::video::prompt_json::validation::PromptTextKind;
use a4n_core::error::AppError;
use a4n_core::prompt_language::PromptOutputLanguage;

pub(in crate::video::prompt_json::validation) fn required_text(
    value: String,
    field: &str,
) -> Result<String, AppError> {
    let text = value.trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(text.to_string())
}

pub(in crate::video::prompt_json::validation) fn required_min_prompt_text(
    value: String,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
    kind: PromptTextKind,
) -> Result<String, AppError> {
    let text = required_text(value, field)?;
    validate_prompt_text_density(&text, field, prompt_output_language, kind)?;
    validate_direct_prompt_text(&text, field)?;
    Ok(text)
}

pub(in crate::video::prompt_json::validation) fn validate_optional_prompt_text(
    value: &str,
    field: &str,
) -> Result<(), AppError> {
    let text = value.trim();
    if contains_structured_prompt_text(text) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 不能包含 JSON、字典、数组或字段名片段"
        )));
    }
    validate_direct_prompt_text(text, field)?;
    Ok(())
}

fn validate_prompt_text_density(
    text: &str,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
    kind: PromptTextKind,
) -> Result<(), AppError> {
    match prompt_output_language {
        PromptOutputLanguage::ZhCn => validate_min_chars(text, field, zh_min_chars(kind)),
        PromptOutputLanguage::En => validate_min_english_density(
            text,
            field,
            english_min_words(kind),
            english_min_chars(kind),
        ),
    }
}

fn validate_min_chars(text: &str, field: &str, min_chars: usize) -> Result<(), AppError> {
    let char_count = text.chars().count();
    if char_count >= min_chars {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "{field} 过短，至少需要 {min_chars} 个字符，当前 {char_count} 个字符"
    )))
}

fn validate_min_english_density(
    text: &str,
    field: &str,
    min_words: usize,
    min_chars: usize,
) -> Result<(), AppError> {
    let word_count = text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .count();
    let char_count = text.chars().count();
    if word_count >= min_words || char_count >= min_chars {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "{field} 过短，英文输出至少需要 {min_words} 个英文词或 {min_chars} 个字符，当前 {word_count} 个英文词、{char_count} 个字符"
    )))
}

fn zh_min_chars(kind: PromptTextKind) -> usize {
    match kind {
        PromptTextKind::VideoVisual | PromptTextKind::FramePrompt => 80,
        PromptTextKind::VideoMotion => 45,
        PromptTextKind::FrameNegative => 20,
    }
}

fn english_min_words(kind: PromptTextKind) -> usize {
    match kind {
        PromptTextKind::VideoVisual | PromptTextKind::FramePrompt => 60,
        PromptTextKind::VideoMotion => 30,
        PromptTextKind::FrameNegative => 8,
    }
}

fn english_min_chars(kind: PromptTextKind) -> usize {
    match kind {
        PromptTextKind::VideoVisual => 320,
        PromptTextKind::VideoMotion => 180,
        PromptTextKind::FramePrompt => 350,
        PromptTextKind::FrameNegative => 60,
    }
}

fn contains_structured_prompt_text(text: &str) -> bool {
    text.contains('{')
        || text.contains('}')
        || text.contains("[{")
        || text.contains("}]")
        || text.contains("\":")
        || text.contains("':")
}
