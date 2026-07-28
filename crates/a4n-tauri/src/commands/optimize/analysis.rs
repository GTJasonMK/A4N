use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, HostErrorResponse},
    optimize::StartOptimizeAnalysisGenerationInput,
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::optimize::{
    self as optimize_actions, AnalyzeChapterInput, OptimizationSessionSummary,
    PrepareOptimizationAnalysisSessionInput,
};
use tauri::State;

use super::adapters::{optimize_input_command, optimize_preview_command};

optimize_input_command!(
    create_optimization_analysis_session,
    AnalyzeChapterInput,
    OptimizationSessionSummary,
    optimize_actions::create_optimization_analysis_session,
);
optimize_input_command!(
    prepare_optimization_analysis_session,
    PrepareOptimizationAnalysisSessionInput,
    OptimizationSessionSummary,
    optimize_actions::prepare_optimization_analysis_session,
);
#[tauri::command]
pub async fn start_optimize_analysis_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartOptimizeAnalysisGenerationInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    optimize_actions::start_optimize_analysis_generation_typed(state.inner(), user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}
optimize_preview_command!(
    preview_create_optimization_analysis_session_request,
    AnalyzeChapterInput,
    optimize_actions::preview_create_optimization_analysis_session_request,
);
optimize_preview_command!(
    preview_prepare_optimization_analysis_session_request,
    PrepareOptimizationAnalysisSessionInput,
    optimize_actions::preview_prepare_optimization_analysis_session_request,
);
