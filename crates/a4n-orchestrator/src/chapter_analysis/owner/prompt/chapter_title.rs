use a4n_core::{error::AppError, models::chapter::chapter};

use super::super::normalization::analysis_schema_error;

pub fn required_chapter_title(ch: &chapter::Model) -> Result<&str, AppError> {
    ch.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| {
            analysis_schema_error(format!(
                "第{}章缺少章节标题，不能执行章节分析",
                ch.chapter_number
            ))
        })
}
