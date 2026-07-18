use crate::{
    AppState, actions::request_preview::ModelRequestPreview, prompt_renderer::RuntimePromptRenderer,
};

use super::super::{
    AnalyzeChapterInput, PrepareOptimizationAnalysisSessionInput,
    access::{
        find_chapter_by_id, find_chapter_for_optimize, optimize_session,
        verify_optimize_project_access, verify_optimize_session_access,
    },
    preview::optimization_analysis_request_preview,
    variables::optimization_analysis_variables,
};

pub async fn preview_create_optimization_analysis_session_request(
    state: &AppState,
    user_id: i64,
    input: AnalyzeChapterInput,
) -> Result<ModelRequestPreview, String> {
    verify_optimize_project_access(state, user_id, input.project_id).await?;
    let chapter = find_chapter_for_optimize(state, input.project_id, input.chapter_number).await?;
    let variables = optimization_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        input.permissions.as_ref(),
        input.agent_instruction.as_deref(),
    );
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let request = state
        .build_analysis_preview_request(
            input.project_id,
            chapter.id,
            &chapter.content,
            &prompt_renderer,
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
    verify_optimize_session_access(state, user_id, input.session_id).await?;
    let current_session = optimize_session(state, input.session_id).await?;
    let chapter = find_chapter_by_id(state, current_session.chapter_id).await?;
    let variables = optimization_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        input.permissions.as_ref(),
        input.agent_instruction.as_deref(),
    );
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let request = state
        .build_analysis_preview_request(
            current_session.project_id,
            current_session.chapter_id,
            &chapter.content,
            &prompt_renderer,
            &variables,
        )
        .await
        .map_err(|error| error.to_string())?;
    optimization_analysis_request_preview(state, request).await
}
