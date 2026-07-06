use super::fields::{
    optional_array, optional_f32, optional_positive_i32, required_array, required_string,
};
use a4n_core::error::AppError;
use serde_json::Value;

pub(in crate::outline) fn parse_chapter_outline_batch(
    content: &str,
    start_chapter: i32,
    count: i32,
) -> Result<Vec<Value>, AppError> {
    let json = parse_outline_json_object(content, "outline")?;
    reject_keys(&json, &["chapter_outline"], "outline")?;
    reject_unknown_keys(&json, &["chapters"], "outline")?;

    let chapters = required_array(&json, "chapters")?;
    let chapters = chapters
        .as_array()
        .expect("required_array guarantees JSON array");
    let expected_count = usize::try_from(count)
        .map_err(|_| AppError::InvalidArgument("章节大纲生成 count 超出 usize 范围".to_string()))?;
    if chapters.len() != expected_count {
        return Err(AppError::InvalidArgument(format!(
            "outline.chapters 必须包含 {expected_count} 个章节，实际为 {} 个",
            chapters.len()
        )));
    }

    chapters
        .iter()
        .enumerate()
        .map(|(idx, chapter)| {
            let expected_chapter = start_chapter + idx as i32;
            materialize_chapter_outline_item(chapter, expected_chapter)
        })
        .collect()
}

fn materialize_chapter_outline_item(
    chapter: &Value,
    expected_chapter: i32,
) -> Result<Value, AppError> {
    reject_unknown_keys(
        chapter,
        &[
            "title",
            "summary",
            "beats",
            "key_scenes",
            "pacing_score",
            "foreshadowing_setup",
            "foreshadowing_payoff",
            "characters_involved",
            "word_count_target",
        ],
        "outline.chapters[]",
    )?;
    required_string(chapter, "title")?;
    required_string(chapter, "summary")?;
    optional_array(chapter, "beats")?;
    optional_array(chapter, "key_scenes")?;
    optional_f32(chapter, "pacing_score")?;
    optional_array(chapter, "foreshadowing_setup")?;
    optional_array(chapter, "foreshadowing_payoff")?;
    optional_array(chapter, "characters_involved")?;
    optional_positive_i32(chapter, "word_count_target")?;
    let mut materialized = chapter.as_object().cloned().ok_or_else(|| {
        AppError::InvalidArgument("outline.chapters[] 必须是 JSON 对象".to_string())
    })?;
    materialized.insert(
        "chapter_number".to_string(),
        Value::Number(expected_chapter.into()),
    );
    Ok(Value::Object(materialized))
}

pub(in crate::outline) fn parse_outline_json_object(
    content: &str,
    prompt_name: &str,
) -> Result<Value, AppError> {
    let trimmed = content.trim();
    let value: Value = serde_json::from_str(trimmed).map_err(|e| {
        AppError::InvalidArgument(format!(
            "模型返回无法解析为 {prompt_name} JSON: {e}；输出预览：{}",
            json_error_preview(trimmed)
        ))
    })?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{prompt_name} 结果必须是 JSON 对象"
        )));
    }
    Ok(value)
}

fn json_error_preview(content: &str) -> String {
    if content.is_empty() {
        return "<empty>".to_string();
    }
    content.chars().take(240).collect()
}

pub(in crate::outline) fn reject_keys(
    json: &Value,
    keys: &[&str],
    context: &str,
) -> Result<(), AppError> {
    for key in keys {
        if json.get(*key).is_some() {
            return Err(AppError::InvalidArgument(format!(
                "{context} 不再接受字段 {key}"
            )));
        }
    }
    Ok(())
}

pub(in crate::outline) fn reject_unknown_keys(
    json: &Value,
    allowed: &[&str],
    context: &str,
) -> Result<(), AppError> {
    let object = json
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 必须是 JSON 对象")))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 包含未知字段 {key}"
            )));
        }
    }
    Ok(())
}
