use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::batch::parse_chapter_outline_batch;

pub(in crate::outline) fn validate_materialized_chapter_outline_batch(
    value: &Value,
    start_chapter: i32,
    count: i32,
) -> Result<Vec<Value>, AppError> {
    let chapters = value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("章节大纲断点 parsed_output 必须是数组".to_string())
    })?;
    let mut raw_chapters = Vec::with_capacity(chapters.len());
    for (index, chapter) in chapters.iter().enumerate() {
        let mut raw = chapter.as_object().cloned().ok_or_else(|| {
            AppError::InvalidArgument(format!("章节大纲断点 parsed_output[{index}] 必须是对象"))
        })?;
        let expected = start_chapter
            .checked_add(i32::try_from(index).map_err(|_| {
                AppError::InvalidArgument("章节大纲断点索引超过 i32 范围".to_string())
            })?)
            .ok_or_else(|| AppError::InvalidArgument("章节大纲断点章节号溢出".to_string()))?;
        let actual = raw
            .remove("chapter_number")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "章节大纲断点 parsed_output[{index}] 缺少整数 chapter_number"
                ))
            })?;
        if actual != i64::from(expected) {
            return Err(AppError::InvalidArgument(format!(
                "章节大纲断点 parsed_output[{index}].chapter_number 必须等于 {expected}"
            )));
        }
        raw_chapters.push(Value::Object(raw));
    }
    parse_chapter_outline_batch(
        &serde_json::to_string(&json!({"chapters": raw_chapters}))?,
        start_chapter,
        count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_materialized_checkpoint_and_reapplies_canonical_rules() {
        let parsed = validate_materialized_chapter_outline_batch(
            &json!([
                {"chapter_number": 3, "title": "暗线浮出", "summary": "发现线索"},
                {"chapter_number": 4, "title": "旧账重启", "summary": "开始反制"}
            ]),
            3,
            2,
        )
        .expect("canonical checkpoint must pass");

        assert_eq!(parsed[0]["chapter_number"], json!(3));
        assert_eq!(parsed[1]["chapter_number"], json!(4));
    }

    #[test]
    fn rejects_checkpoint_chapter_number_drift() {
        let err = validate_materialized_chapter_outline_batch(
            &json!([{"chapter_number": 4, "title": "暗线浮出", "summary": "发现线索"}]),
            3,
            1,
        )
        .expect_err("drifted chapter number must fail");

        assert!(err.to_string().contains("chapter_number 必须等于 3"));
    }
}
