use crate::AppState;
use a4n_core::{error::AppError, models::chapter::chapter};
use a4n_orchestrator::chapter_content::{ChapterContentEditError, ChapterContentImportError};
use a4n_orchestrator::project::PROJECT_KIND_TEXT;

pub(super) fn chapter_content_import_error_to_app_error(
    error: ChapterContentImportError,
) -> AppError {
    match error {
        ChapterContentImportError::Database(error) => AppError::Database(error),
        ChapterContentImportError::PreviousChapterMissing {
            previous_number,
            chapter_number,
        } => AppError::InvalidArgument(format!(
            "请先完成第{previous_number}章正文后，再导入第{chapter_number}章"
        )),
        ChapterContentImportError::PreviousChapterGenerating {
            previous_number,
            chapter_number,
            status,
        } => AppError::InvalidState {
            current: status,
            operation: format!("等待第{previous_number}章完成后导入第{chapter_number}章"),
        },
        ChapterContentImportError::PreviousChapterNoContent {
            previous_number,
            chapter_number,
        } => AppError::InvalidArgument(format!(
            "第{previous_number}章尚无可用正文，请先生成或导入正文后再导入第{chapter_number}章"
        )),
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

pub(super) fn chapter_content_edit_error_to_app_error(error: ChapterContentEditError) -> AppError {
    match error {
        ChapterContentEditError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ChapterContentEditError::NotFound(message) => AppError::NotFound(message),
        ChapterContentEditError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ChapterContentEditError::Database(error) => AppError::Database(error),
    }
}

pub(super) fn ensure_import_chapter_number(chapter_number: i32) -> Result<(), AppError> {
    if chapter_number <= 0 {
        return Err(AppError::InvalidArgument("章节序号必须大于 0".to_string()));
    }
    Ok(())
}

pub(super) fn ensure_chapter_content_not_empty(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::InvalidArgument("章节正文不能为空".to_string()));
    }
    Ok(())
}

pub(super) async fn ensure_text_source_project(
    state: &AppState,
    project_id: i64,
    operation: &str,
) -> Result<(), AppError> {
    let project = state
        .orchestrator
        .project_by_id(project_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(AppError::InvalidArgument(format!(
            "{operation}只能作用于文本项目"
        )));
    }
    Ok(())
}

pub(super) fn ensure_chapter_not_generating(
    ch: &chapter::Model,
    operation: &str,
) -> Result<(), AppError> {
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
