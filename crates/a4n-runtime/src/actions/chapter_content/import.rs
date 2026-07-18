use super::{ImportChapterInput, ImportedChapterSummary, rules};
use crate::{AppState, actions::access};
use a4n_core::{error::AppError, models::chapter::chapter::STATUS_IMPORTED};
use a4n_orchestrator::chapter_content::ImportChapterContentInput as DbImportChapterContentInput;

pub async fn import_chapter(
    state: &AppState,
    user_id: i64,
    input: ImportChapterInput,
) -> Result<ImportedChapterSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    rules::ensure_text_source_project(state, input.project_id, "导入章节正文").await?;
    rules::ensure_import_chapter_number(input.chapter_number)?;
    rules::ensure_chapter_content_not_empty(&input.content)?;

    let word_count = i32::try_from(input.content.chars().count())
        .map_err(|_| AppError::InvalidArgument("导入章节正文字符数超出 i32 范围".to_string()))?;
    let now = chrono::Utc::now().naive_utc();
    let normalized_title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string);

    state
        .orchestrator
        .prepare_import_chapter_content(input.project_id, input.chapter_number, "导入覆盖正文")
        .await
        .map_err(rules::chapter_content_import_error_to_app_error)?;
    crate::workflow::preflight_chapter_content_change(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;

    let chapter = state
        .orchestrator
        .import_chapter_content(
            DbImportChapterContentInput {
                project_id: input.project_id,
                chapter_number: input.chapter_number,
                content: input.content,
                normalized_title,
                word_count,
                now,
            },
            "导入覆盖正文",
        )
        .await
        .map_err(rules::chapter_content_import_error_to_app_error)?;

    crate::workflow::postprocess_chapter_after_content_change(state, chapter.id).await?;

    Ok(ImportedChapterSummary {
        id: chapter.id,
        project_id: chapter.project_id,
        chapter_number: chapter.chapter_number,
        title: chapter.title,
        status: STATUS_IMPORTED.to_string(),
        word_count,
    })
}
