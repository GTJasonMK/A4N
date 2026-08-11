use crate::AppState;
use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, optimize::StartOptimizeAnalysisGenerationInput,
};
use a4n_orchestrator::GenerationTaskAccessError;
use a4n_orchestrator::optimize::{
    PrepareOptimizeAnalysisApplicationInput, optimization_analysis_variables,
};

use super::{
    AnalyzeChapterInput, OptimizationSessionSummary, PrepareOptimizationAnalysisSessionInput,
    contracts::orchestrator_permissions, projections::to_session_summary,
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
        .orchestrator()
        .start_optimize_analysis_generation_task(user_id, input.clone())
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }
    let worker_state = state.clone();
    let handle = start.handle.clone();
    state.spawn_generation_execution(handle.task_id.clone(), async move {
        if let Err(error) = worker_state
            .orchestrator()
            .run_optimize_analysis_generation_task(user_id, input, handle.clone())
            .await
        {
            tracing::error!(
                task_id = %handle.task_id,
                attempt = handle.attempt,
                error = %error,
                "内容优化分析 generation worker 失败"
            );
        }
    });
    Ok(start.handle)
}

pub async fn create_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    input: AnalyzeChapterInput,
) -> Result<OptimizationSessionSummary, String> {
    let permissions = input.permissions.as_ref().map(orchestrator_permissions);
    let variables = optimization_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        permissions.as_ref(),
        input.agent_instruction.as_deref(),
    );
    let prompt_renderer = state;
    let session = state
        .orchestrator()
        .create_optimize_analysis_session_for_user_validated(
            user_id,
            input.project_id,
            input.chapter_number,
            &input.name,
            prompt_renderer,
            &variables,
        )
        .await
        .map_err(|err| err.to_string())?;
    Ok(to_session_summary(&session))
}

pub async fn prepare_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    input: PrepareOptimizationAnalysisSessionInput,
) -> Result<OptimizationSessionSummary, String> {
    let session_id = input.session_id;
    let permissions = input.permissions.as_ref().map(orchestrator_permissions);
    let variables = optimization_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        permissions.as_ref(),
        input.agent_instruction.as_deref(),
    );
    let prompt_renderer = state;
    state
        .orchestrator()
        .prepare_optimize_analysis_session_validated(PrepareOptimizeAnalysisApplicationInput {
            user_id,
            session_id,
            prompt_renderer: prompt_renderer,
            variables: &variables,
        })
        .await
        .map(|session| to_session_summary(&session))
        .map_err(|error| error.to_string())
}
