use a4n_core::models::project::import_review_queue;
use a4n_orchestrator::import_analysis::{
    StartImportAnalysisInput, ingest::create_import_analysis_reviews,
};

use crate::{actions::access, state::AppState};

pub async fn start_import_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: StartImportAnalysisInput,
) -> Result<Vec<import_review_queue::Model>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    create_import_analysis_reviews(&state.orchestrator, project_id, input).await
}
