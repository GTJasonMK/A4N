//! 章节正文 Host 输入与状态门禁的纯规则（无 DB）。

use a4n_core::{error::AppError, models::chapter::chapter};
use a4n_db::chapter_content::{ChapterContentEditError, ChapterContentImportError};

use crate::project::PROJECT_KIND_TEXT;

/// 导入章节序号必须为正。
pub fn ensure_import_chapter_number(chapter_number: i32) -> Result<(), AppError> {
    if chapter_number <= 0 {
        return Err(AppError::InvalidArgument("章节序号必须大于 0".to_string()));
    }
    Ok(())
}

/// 章节正文 trim 后不能为空。
pub fn ensure_chapter_content_not_empty(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::InvalidArgument("章节正文不能为空".to_string()));
    }
    Ok(())
}

/// 项目 kind 必须为文本项目。
pub fn ensure_text_project_kind(project_kind: &str, operation: &str) -> Result<(), AppError> {
    if project_kind != PROJECT_KIND_TEXT {
        return Err(AppError::InvalidArgument(format!(
            "{operation}只能作用于文本项目"
        )));
    }
    Ok(())
}

/// 章节不得处于生成中状态。
pub fn ensure_chapter_not_generating(ch: &chapter::Model, operation: &str) -> Result<(), AppError> {
    if ch.is_generating() {
        return Err(AppError::InvalidState {
            current: ch.status.clone(),
            operation: format!(
                "第 {} 章正在生成中，请等待当前生成完成后再{operation}",
                ch.chapter_number
            ),
        });
    }
    Ok(())
}

/// 导入错误映射为 AppError。
pub(crate) fn chapter_content_import_error_to_app_error(
    error: ChapterContentImportError,
) -> AppError {
    match error {
        ChapterContentImportError::Database(error) => AppError::Database(error),
        ChapterContentImportError::TargetChapterGenerating {
            chapter_number,
            status,
            operation,
        } => AppError::InvalidState {
            current: status,
            operation: format!(
                "第 {chapter_number} 章正在生成中，请等待当前生成完成后再{operation}"
            ),
        },
    }
}

/// 编辑错误映射为 AppError。
pub(crate) fn chapter_content_edit_error_to_app_error(error: ChapterContentEditError) -> AppError {
    match error {
        ChapterContentEditError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ChapterContentEditError::NotFound(message) => AppError::NotFound(message),
        ChapterContentEditError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ChapterContentEditError::Database(error) => AppError::Database(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_import_chapter_number_rejects_non_positive() {
        let err = ensure_import_chapter_number(0).expect_err("must be positive");
        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message == "章节序号必须大于 0"
        ));
        assert!(ensure_import_chapter_number(1).is_ok());
    }

    #[test]
    fn ensure_chapter_content_not_empty_rejects_blank() {
        let err = ensure_chapter_content_not_empty(" \n\t ").expect_err("blank");
        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message == "章节正文不能为空"
        ));
        assert!(ensure_chapter_content_not_empty("有效正文").is_ok());
    }

    #[test]
    fn ensure_text_project_kind_rejects_non_text() {
        assert!(ensure_text_project_kind(PROJECT_KIND_TEXT, "导入").is_ok());
        let err = ensure_text_project_kind("video", "导入").expect_err("non-text");
        assert!(err.to_string().contains("只能作用于文本项目"));
    }
}
