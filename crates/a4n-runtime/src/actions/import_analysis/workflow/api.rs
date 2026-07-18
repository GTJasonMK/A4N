use super::stages::run_import_project_analysis_inner;
use crate::state::AppState;
use a4n_orchestrator::import_analysis::{
    ImportProjectAnalysisSummary,
    workflow::{
        PreparedImportAnalysisRun, prepare_import_project_analysis, record_import_analysis_failure,
    },
};

pub(in crate::actions::import_analysis) async fn run_import_project_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImportProjectAnalysisSummary, String> {
    let mut prepared =
        match prepare_import_project_analysis(&state.orchestrator, project_id).await? {
            PreparedImportAnalysisRun::Completed(summary) => return Ok(summary),
            PreparedImportAnalysisRun::Ready(prepared) => prepared,
        };

    let result = run_import_project_analysis_inner(
        state,
        user_id,
        project_id,
        &prepared.project,
        prepared.chapters,
        prepared.needs_part_outlines,
        &mut prepared.progress,
    )
    .await;

    match result {
        Ok(summary) => Ok(summary),
        Err(err) => {
            record_import_analysis_failure(&state.orchestrator, project_id, &err).await?;
            Err(err)
        }
    }
}
