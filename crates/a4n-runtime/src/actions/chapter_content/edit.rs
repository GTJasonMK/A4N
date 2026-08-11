use super::{ChapterSummary, UpdateChapterContentInput, projections};
use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::models::generation::workflow_run;

pub async fn update_chapter_content(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterContentInput,
) -> Result<ChapterSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let updated = state
        .orchestrator()
        .update_chapter_content_validated(
            user_id,
            &host_port,
            input.chapter_id,
            input.content,
            input.title,
        )
        .await?;
    projections::chapter_summary_with_sync_state(state, &updated).await
}

pub async fn resume_chapter_content_sync_workflow_run(
    state: &AppState,
    run: &workflow_run::Model,
) -> Result<(), String> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .orchestrator()
        .resume_chapter_content_sync_workflow_run_validated(&host_port, run)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}
