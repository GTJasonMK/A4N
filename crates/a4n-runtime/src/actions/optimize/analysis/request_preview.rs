use super::super::{
    AnalyzeChapterInput, PrepareOptimizationAnalysisSessionInput,
    preview::optimization_analysis_request_preview,
};
use crate::{AppState, actions::request_preview::ModelRequestPreview};

pub async fn preview_create_optimization_analysis_session_request(
    state: &AppState,
    user_id: i64,
    input: AnalyzeChapterInput,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let request = state
        .optimize_preview_application()
        .preview_create_analysis(user_id, input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    optimization_analysis_request_preview(state, request).await
}

pub async fn preview_prepare_optimization_analysis_session_request(
    state: &AppState,
    user_id: i64,
    input: PrepareOptimizationAnalysisSessionInput,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let request = state
        .optimize_preview_application()
        .preview_prepare_analysis(user_id, input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    optimization_analysis_request_preview(state, request).await
}
