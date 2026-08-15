use crate::{AppState, actions::request_preview::ModelRequestPreview};
use a4n_orchestrator::optimize::optimization_analysis_variables;

use super::super::{
    AnalyzeChapterInput, PrepareOptimizationAnalysisSessionInput,
    preview::optimization_analysis_request_preview,
};

pub async fn preview_create_optimization_analysis_session_request(
    state: &AppState,
    user_id: i64,
    input: AnalyzeChapterInput,
) -> Result<ModelRequestPreview, String> {
    let variables = optimization_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        &input.permissions,
        input.agent_instruction.as_deref(),
    );
    let prompt_renderer = state;
    let request = state
        .orchestrator()
        .build_create_optimize_analysis_preview_request_for_user_validated(
            user_id,
            input.project_id,
            input.chapter_number,
            prompt_renderer,
            &variables,
        )
        .await
        .map_err(|error| error.to_string())?;
    optimization_analysis_request_preview(state, request).await
}

pub async fn preview_prepare_optimization_analysis_session_request(
    state: &AppState,
    user_id: i64,
    input: PrepareOptimizationAnalysisSessionInput,
) -> Result<ModelRequestPreview, String> {
    let variables = optimization_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        &input.permissions,
        input.agent_instruction.as_deref(),
    );
    let prompt_renderer = state;
    let request = state
        .orchestrator()
        .build_prepare_optimize_analysis_preview_request_for_user_validated(
            user_id,
            input.session_id,
            prompt_renderer,
            &variables,
        )
        .await
        .map_err(|error| error.to_string())?;
    optimization_analysis_request_preview(state, request).await
}
