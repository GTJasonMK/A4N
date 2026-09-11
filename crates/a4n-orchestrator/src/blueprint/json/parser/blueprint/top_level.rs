use a4n_error::AppError;
use serde_json::Value;

use super::super::super::fields::{reject_keys, reject_unknown_keys};

pub(super) fn parse_blueprint_top_level(content: &str) -> Result<Value, AppError> {
    let raw: Value = serde_json::from_str(content.trim())
        .map_err(|e| AppError::InvalidArgument(format!("模型返回无法解析为严格蓝图 JSON: {e}")))?;
    let object = raw
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument("蓝图生成结果必须是 JSON 对象".to_string()))?;
    reject_keys(object, &["novel_title", "chapter_count"], "blueprint")?;
    reject_unknown_keys(
        object,
        &[
            "title",
            "genre",
            "style",
            "tone",
            "target_audience",
            "one_sentence_summary",
            "full_synopsis",
            "total_chapters",
            "needs_part_outlines",
            "chapters_per_part",
            "chapter_outline",
            "world_setting",
            "characters",
            "relationships",
        ],
        "blueprint",
    )?;
    Ok(raw)
}
