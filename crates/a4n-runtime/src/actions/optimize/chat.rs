use crate::{AppState, prompt_renderer::RuntimePromptRenderer};
use a4n_orchestrator::optimize::{OptimizeLlmStream, OptimizeProgress};

use super::{
    CreateOptimizationChatSessionInput, OptimizationSessionSummary, OptimizationTaskScope,
    SendOptimizationMessageInput, SendOptimizationMessageResponse,
    access::{
        find_chapter_for_optimize, optimize_session_task_scope, verify_optimize_project_access,
    },
    projections::{to_session_summary, to_turn_summary},
    variables::optimization_agent_variables,
    workflow::{
        OptimizeCommandError, finalize_optimize_workflow_run, record_optimize_command_result,
        start_optimize_workflow_run,
    },
};

mod request_preview;

pub use request_preview::preview_send_optimization_message_request;

pub async fn send_optimization_message_task_scope(
    state: &AppState,
    user_id: i64,
    input: &SendOptimizationMessageInput,
) -> Result<OptimizationTaskScope, String> {
    optimize_session_task_scope(state, user_id, input.session_id, 4).await
}

pub async fn create_optimization_chat_session(
    state: &AppState,
    user_id: i64,
    input: CreateOptimizationChatSessionInput,
) -> Result<OptimizationSessionSummary, String> {
    verify_optimize_project_access(state, user_id, input.project_id).await?;
    let chapter = find_chapter_for_optimize(state, input.project_id, input.chapter_number).await?;
    let variables = optimization_agent_variables(input.permissions.as_ref());
    let session = state
        .start_chat_session(input.project_id, chapter.id, &input.name, &variables)
        .await
        .map_err(|err| err.to_string())?;
    Ok(to_session_summary(&session))
}

pub async fn send_optimization_message(
    state: &AppState,
    user_id: i64,
    input: SendOptimizationMessageInput,
    mut progress: Option<OptimizeProgress<'_>>,
    stream: Option<OptimizeLlmStream<'_>>,
) -> Result<SendOptimizationMessageResponse, String> {
    let session_id = input.session_id;
    send_optimization_message_task_scope(state, user_id, &input).await?;
    if let Some(progress) = progress.as_mut() {
        progress(0, 4, "正在准备内容优化 Agent 对话").map_err(|err| err.to_string())?;
    }
    let workflow_run_id =
        start_optimize_workflow_run(state, session_id, "content_optimization_chat", "chat").await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let result = match state
        .send_user_message_with_permissions(
            a4n_orchestrator::optimize::OptimizeSendUserMessageInput {
                session_id: input.session_id,
                message: &input.message,
                permissions_override: input.permissions.as_ref(),
                prompt_renderer: &prompt_renderer,
                workflow_run_id,
                progress,
                stream,
            },
        )
        .await
    {
        Ok(turn) => {
            let events = state
                .orchestrator
                .optimization_turn_events(turn.id)
                .await
                .map_err(OptimizeCommandError::from_app);
            events.map(|events| SendOptimizationMessageResponse {
                session_id: input.session_id,
                turn: to_turn_summary(&turn, events),
            })
        }
        Err(err) => Err(OptimizeCommandError::from_app(err)),
    };
    let result =
        finalize_optimize_workflow_run(state, workflow_run_id, "内容优化对话", result).await;
    record_optimize_command_result(state, session_id, "内容优化对话", result).await
}
