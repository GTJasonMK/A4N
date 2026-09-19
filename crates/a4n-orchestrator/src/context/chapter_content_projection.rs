use super::OrchestratorContext;
use a4n_api_contract::chapter::ChapterSummary;
use a4n_db_models::models::chapter::chapter;
use a4n_error::AppError;

pub(crate) fn chapter_summary(chapter: &chapter::Model) -> ChapterSummary {
    a4n_chapter_content_core::chapter_summary(chapter)
}

pub(crate) async fn chapter_summaries_with_sync_state(
    state: &OrchestratorContext,
    chapters: &[chapter::Model],
) -> Result<Vec<ChapterSummary>, AppError> {
    let Some(project_id) = chapters.first().map(|chapter| chapter.project_id) else {
        return Ok(Vec::new());
    };
    let (analysis_runs_result, content_runs_result) = futures::join!(
        state.workflow_runs_by_project_kind(project_id, "chapter_analysis"),
        state.workflow_runs_by_project_kind(
            project_id,
            a4n_chapter_content_core::CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
        ),
    );
    let analysis_runs = analysis_runs_result?;
    let content_runs = content_runs_result?;
    let run_ids = analysis_runs
        .iter()
        .chain(&content_runs)
        .map(|run| run.id)
        .collect::<Vec<_>>();
    let checkpoints = state.workflow_checkpoint_models_for_runs(&run_ids).await?;
    let mut summaries = chapters.iter().map(chapter_summary).collect::<Vec<_>>();
    a4n_chapter_content_core::attach_chapter_sync_state(
        chapters,
        &mut summaries,
        &analysis_runs,
        &content_runs,
        &checkpoints,
    )?;
    Ok(summaries)
}

pub(crate) async fn chapter_summary_with_sync_state(
    state: &OrchestratorContext,
    chapter: &chapter::Model,
) -> Result<ChapterSummary, AppError> {
    let mut summaries =
        chapter_summaries_with_sync_state(state, std::slice::from_ref(chapter)).await?;
    summaries.pop().ok_or_else(|| AppError::InvalidState {
        current: "empty_chapter_summary_projection".to_string(),
        operation: "读取章节摘要".to_string(),
    })
}
