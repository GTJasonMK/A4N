use super::ImportProjectAnalysisSummary;
use crate::actions::access;
use crate::actions::character_ai::RuntimeCharacterProfileSyncStageTracePort;
use crate::prompt_renderer::RuntimePromptRenderer;
use crate::state::AppState;
use crate::workflow::RuntimeChapterAnalysisHostPort;
use serde_json::Value;

pub async fn get_project_import_analysis_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Option<Value>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .import_analysis()
        .project_import_analysis_progress(project_id)
        .await
}

pub async fn run_import_project_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImportProjectAnalysisSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let host_port = RuntimeChapterAnalysisHostPort::new(state);
    let trace_port = RuntimeCharacterProfileSyncStageTracePort::new(state.log_dir());
    state
        .import_analysis()
        .run_import_project_analysis_validated(
            project_id,
            &prompt_renderer,
            &host_port,
            Some(&trace_port),
        )
        .await
        .map_err(|error| error.to_string())
}

pub async fn cancel_import_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<u64, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .import_analysis()
        .cancel_import_analysis(project_id)
        .await
}
