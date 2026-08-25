use a4n_core::error::AppError;
use serde_json::Value;

use super::super::fields::{
    optional_array, optional_f32, optional_positive_i32, required_array, required_string,
};
use super::json::{parse_outline_json_object, reject_keys, reject_unknown_keys};

pub(in crate::outline) fn parse_chapter_outline_batch(
    content: &str,
    start_chapter: i32,
    count: i32,
) -> Result<Vec<Value>, AppError> {
    let json = parse_outline_json_object(content, "outline")?;
    reject_keys(&json, &["chapter_outline"], "outline")?;
    reject_unknown_keys(&json, &["chapters"], "outline")?;

    let chapters_value = required_array(&json, "chapters")?;
    let chapters = chapters_value
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("outline.chapters 必须是数组".to_string()))?;
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
        .map(|(index, chapter)| {
            let chapter_offset = i32::try_from(index).map_err(|_| {
                AppError::InvalidArgument(format!("章节大纲索引超过 i32 范围：{index}"))
            })?;
            let expected_chapter = start_chapter.checked_add(chapter_offset).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "章节大纲编号超过 i32 范围：{start_chapter} + {chapter_offset}"
                ))
            })?;
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
