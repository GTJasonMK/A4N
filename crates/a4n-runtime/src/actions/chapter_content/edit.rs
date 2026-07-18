use super::{ChapterSummary, UpdateChapterContentInput, projections, rules};
use crate::{AppState, actions::access};
use a4n_core::error::AppError;
use a4n_orchestrator::chapter_content::EditChapterContentInput as DbEditChapterContentInput;

pub async fn update_chapter_content(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterContentInput,
) -> Result<ChapterSummary, AppError> {
    let chapter = state
        .orchestrator
        .chapter_by_id(input.chapter_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", input.chapter_id)))?;
    access::verify_project_editor(state, user_id, chapter.project_id).await?;
    rules::ensure_text_source_project(state, chapter.project_id, "更新章节正文").await?;
    rules::ensure_chapter_not_generating(&chapter, "更新章节正文")?;
    rules::ensure_chapter_content_not_empty(&input.content)?;
    crate::workflow::preflight_chapter_content_change(
        state,
        chapter.project_id,
        chapter.chapter_number,
    )
    .await?;
    let title = input.title.as_deref().map(str::to_string);
    let updated = state
        .orchestrator
        .save_chapter_content_edit(DbEditChapterContentInput {
            chapter_id: input.chapter_id,
            content: input.content,
            title,
        })
        .await
        .map_err(rules::chapter_content_edit_error_to_app_error)?;

    crate::workflow::postprocess_chapter_after_content_change(state, updated.id).await?;
    let refreshed = state
        .orchestrator
        .chapter_by_id(updated.id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", updated.id)))?;
    Ok(projections::chapter_summary(&refreshed))
}
