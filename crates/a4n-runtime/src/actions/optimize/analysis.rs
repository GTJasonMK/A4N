use crate::AppState;
use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, optimize::StartOptimizeAnalysisGenerationInput,
};
use a4n_orchestrator::GenerationTaskAccessError;

use super::{
    AnalyzeChapterInput, OptimizationSessionSummary, PrepareOptimizationAnalysisSessionInput,
};

mod request_preview;

pub use request_preview::{
    preview_create_optimization_analysis_session_request,
    preview_prepare_optimization_analysis_session_request,
};

pub async fn start_optimize_analysis_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartOptimizeAnalysisGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .optimize_application()
        .start_analysis_generation(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }
    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        "optimize_analysis",
        move |handle| async move {
            worker_state
                .optimize_application()
                .run_analysis_generation(user_id, input, handle)
                .await
        },
    );
    Ok(start.handle)
}

pub async fn create_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    input: AnalyzeChapterInput,
) -> Result<OptimizationSessionSummary, String> {
    let prompt_renderer = state;
    let session = state
        .optimize_session_mutation_application()
        .create_analysis_session(user_id, input, prompt_renderer)
        .await
        .map_err(|err| err.to_string())?;
    Ok(session)
}

pub async fn prepare_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    input: PrepareOptimizationAnalysisSessionInput,
) -> Result<OptimizationSessionSummary, String> {
    let prompt_renderer = state;
    let session = state
        .optimize_session_mutation_application()
        .prepare_analysis_session(user_id, input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    Ok(session)
}
