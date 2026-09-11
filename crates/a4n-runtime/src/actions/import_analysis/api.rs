use super::{ImportProjectAnalysisSummary, ProjectImportAnalysisProgress};
use crate::actions::character_ai::RuntimeCharacterProfileSyncStageTracePort;
use crate::state::AppState;
use crate::workflow::RuntimeChapterAnalysisHostPort;
use a4n_error::AppError;

pub async fn get_project_import_analysis_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Option<ProjectImportAnalysisProgress>, AppError> {
    state
        .import_analysis_application()
        .progress(user_id, project_id)
        .await
}

pub async fn run_import_project_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImportProjectAnalysisSummary, AppError> {
    let prompt_renderer = state;
    let host_port = RuntimeChapterAnalysisHostPort::new(state);
    let trace_port = RuntimeCharacterProfileSyncStageTracePort::new(state.log_dir());
    state
        .import_analysis_application()
        .run(
            user_id,
            project_id,
            prompt_renderer,
            &host_port,
            Some(&trace_port),
        )
        .await
}

pub async fn cancel_import_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    state
        .import_analysis_application()
        .cancel(user_id, project_id)
        .await
}
