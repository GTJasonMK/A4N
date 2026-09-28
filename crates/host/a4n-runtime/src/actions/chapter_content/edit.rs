use super::{ChapterSummary, UpdateChapterContentInput};
use crate::AppState;
use a4n_error::AppError;

pub async fn update_chapter_content(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterContentInput,
) -> Result<ChapterSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let chapter_id = state
        .chapter_content_mutation_application()
        .update(user_id, &host_port, input)
        .await?;
    state
        .chapter_content_read_application()
        .summary(user_id, chapter_id, None)
        .await
}

pub(crate) async fn resume_chapter_content_sync_workflow_run(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<(), String> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .chapter_content_sync_application()
        .resume(&host_port, workflow_run_id)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}
