use super::ImportProjectAnalysisSummary;
use super::workflow;
use crate::actions::access;
use crate::state::AppState;
use a4n_orchestrator::import_analysis::lifecycle;
use serde_json::Value;

pub async fn get_project_import_analysis_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Option<Value>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    lifecycle::project_import_analysis_progress(&state.orchestrator, project_id).await
}

pub async fn run_import_project_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImportProjectAnalysisSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    workflow::run_import_project_analysis(state, user_id, project_id).await
}

pub async fn cancel_import_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<u64, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    lifecycle::cancel_import_analysis(&state.orchestrator, project_id).await
}
