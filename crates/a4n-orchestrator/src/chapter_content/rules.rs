//! 章节正文 Host 输入与状态门禁的纯规则（无 DB）。

use a4n_core::error::AppError;
use a4n_db::chapter_content::ChapterContentImportError;

use crate::project::PROJECT_KIND_TEXT;

/// 项目 kind 必须为文本项目。
pub(super) fn ensure_text_project_kind(
    project_kind: &str,
    operation: &str,
) -> Result<(), AppError> {
    if project_kind != PROJECT_KIND_TEXT {
        return Err(AppError::InvalidArgument(format!(
            "{operation}只能作用于文本项目"
        )));
    }
    Ok(())
}

/// 导入错误映射为 AppError。
pub(crate) fn chapter_content_import_error_to_app_error(
    error: ChapterContentImportError,
) -> AppError {
    match error {
        ChapterContentImportError::InvalidArgument(message) => AppError::InvalidArgument(message),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_text_project_kind_rejects_non_text() {
        assert!(ensure_text_project_kind(PROJECT_KIND_TEXT, "导入").is_ok());
        let err = ensure_text_project_kind("video", "导入").expect_err("non-text");
        assert!(err.to_string().contains("只能作用于文本项目"));
    }
}
