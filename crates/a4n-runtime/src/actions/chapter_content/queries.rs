use super::{ChapterSummary, projections};
use crate::{AppState, actions::access};
use a4n_core::error::AppError;

pub async fn list_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterSummary>, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    let chapters = state
        .chapter_content()
        .chapters_for_project(project_id)
        .await?;
    let mut summaries = chapters
        .iter()
        .map(projections::chapter_summary)
        .collect::<Vec<_>>();
    projections::attach_chapter_analysis_sync_state(state, &chapters, &mut summaries).await?;
    Ok(summaries)
}
