use crate::{AppState, prompt_renderer::RuntimePromptRenderer};
use a4n_orchestrator::optimize::{OptimizeLlmStream, OptimizeProgress};

use super::{
    AnalyzeChapterInput, OptimizationSessionSummary, OptimizationTaskScope,
    PrepareOptimizationAnalysisSessionInput,
    access::{
        find_chapter_by_id, find_chapter_for_optimize, optimize_session,
        optimize_session_task_scope, verify_optimize_project_access,
        verify_optimize_session_access,
    },
    projections::to_session_summary,
    variables::optimization_analysis_variables,
    workflow::{
        OptimizeCommandError, finalize_optimize_workflow_run, record_optimize_command_result,
        start_optimize_workflow_run,
    },
};

mod request_preview;

pub use request_preview::{
    preview_create_optimization_analysis_session_request,
    preview_prepare_optimization_analysis_session_request,
};

pub async fn execute_optimization_analysis_session_task_scope(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<OptimizationTaskScope, String> {
    optimize_session_task_scope(state, user_id, session_id, 5).await
}

pub async fn create_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    input: AnalyzeChapterInput,
) -> Result<OptimizationSessionSummary, String> {
    verify_optimize_project_access(state, user_id, input.project_id).await?;
    let chapter = find_chapter_for_optimize(state, input.project_id, input.chapter_number).await?;
    let variables = optimization_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        input.permissions.as_ref(),
        input.agent_instruction.as_deref(),
    );
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let session = state
        .create_analysis_session(
            input.project_id,
            chapter.id,
            &input.name,
            &chapter.content,
            &prompt_renderer,
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
    let result = async {
        verify_optimize_session_access(state, user_id, input.session_id)
            .await
            .map_err(OptimizeCommandError::from_message)?;
        let current_session = optimize_session(state, input.session_id)
            .await
            .map_err(OptimizeCommandError::from_message)?;
        let chapter = find_chapter_by_id(state, current_session.chapter_id)
            .await
            .map_err(OptimizeCommandError::from_message)?;
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            input.permissions.as_ref(),
            input.agent_instruction.as_deref(),
        );
        let prompt_renderer = RuntimePromptRenderer::new(state);
        let session = state
            .prepare_analysis_session(session_id, &chapter.content, &prompt_renderer, &variables)
            .await
            .map_err(OptimizeCommandError::from_app)?;
        Ok(to_session_summary(&session))
    }
    .await;
    record_optimize_command_result(state, session_id, "准备内容优化分析", result).await
}

pub async fn execute_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    session_id: i64,
    progress: Option<OptimizeProgress<'_>>,
    stream: Option<OptimizeLlmStream<'_>>,
) -> Result<OptimizationSessionSummary, String> {
    execute_optimization_analysis_session_task_scope(state, user_id, session_id).await?;
    let workflow_run_id = start_optimize_workflow_run(
        state,
        session_id,
        "content_optimization_analysis",
        "analysis",
    )
    .await?;
    let result = state
        .run_analysis_session(session_id, workflow_run_id, progress, stream)
        .await
        .map(|session| to_session_summary(&session))
        .map_err(OptimizeCommandError::from_app);
    let result =
        finalize_optimize_workflow_run(state, workflow_run_id, "内容优化分析", result).await;
    record_optimize_command_result(state, session_id, "内容优化分析", result).await
}
