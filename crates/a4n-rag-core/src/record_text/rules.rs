use serde_json::Value;

use super::json::format_json_for_rag;

pub fn required_rag_text(value: Option<&str>, context: impl Into<String>) -> Result<&str, String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| context.into())
}

pub fn optional_rag_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}

pub fn optional_json_text(value: Option<&Value>) -> Option<String> {
    value
        .map(format_json_for_rag)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub fn short_rag_title(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut title = trimmed.chars().take(max_chars).collect::<String>();
    title.push_str("...");
    title
}
