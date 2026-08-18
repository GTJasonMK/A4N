use crate::AppState;
use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::optimize::StartOptimizeChatGenerationInput;
use a4n_orchestrator::GenerationTaskAccessError;
use a4n_orchestrator::optimize::optimization_agent_variables;

use super::{
    CreateOptimizationChatSessionInput, OptimizationSessionSummary, projections::to_session_summary,
};

mod request_preview;

pub use request_preview::preview_send_optimization_message_request;

pub async fn start_optimize_chat_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartOptimizeChatGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .orchestrator()
        .start_optimize_chat_generation_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        "optimize_chat",
        move |handle| async move {
            let prompt_renderer = &worker_state;
            worker_state
                .orchestrator()
                .run_optimize_chat_generation_task(
                    a4n_orchestrator::optimize::OptimizeChatGenerationApplicationContext {
                        prompt_renderer: prompt_renderer,
                    },
                    user_id,
                    input,
                    handle,
                )
                .await
        },
    );
    Ok(start.handle)
}

pub async fn create_optimization_chat_session(
    state: &AppState,
    user_id: i64,
    input: CreateOptimizationChatSessionInput,
) -> Result<OptimizationSessionSummary, String> {
    let variables = optimization_agent_variables(&input.permissions);
    let session = state
        .orchestrator()
        .create_optimize_chat_session_for_user_validated(
            user_id,
            input.project_id,
            input.chapter_number,
            &input.name,
            &variables,
        )
        .await
        .map_err(|err| err.to_string())?;
    to_session_summary(&session)
}
