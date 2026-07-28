use super::super::{ChapterOutlineSummary, UpdateChapterOutlineInput, chapter_outline_summary};
use crate::AppState;
use crate::actions::access as project_access;
use a4n_core::error::AppError;
use a4n_orchestrator::project::ProjectChapterOutlineUpdateInput;

pub async fn list_chapter_outlines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    project_access::verify_project_access(state, user_id, project_id).await?;
    let outlines = state
        .project_outline_records()
        .chapter_outlines_for_project(project_id)
        .await?;
    Ok(outlines.iter().map(chapter_outline_summary).collect())
}

pub async fn delete_latest_chapter_outlines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<(), AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .chapter_content()
        .delete_latest_chapter_outlines_validated(&host_port, project_id, count)
        .await
}

pub async fn update_chapter_outline(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterOutlineInput,
) -> Result<ChapterOutlineSummary, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let updated = state
        .project_outline_records()
        .update_chapter_outline_validated(ProjectChapterOutlineUpdateInput {
            project_id: input.project_id,
            outline_id: input.outline_id,
            title: input.title,
            summary: input.summary,
            beats: input.beats,
            key_scenes: input.key_scenes,
        })
        .await?;
    Ok(chapter_outline_summary(&updated))
}
