use crate::AppState;
use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::optimize::StartOptimizeChatGenerationInput;
use a4n_orchestrator::GenerationTaskAccessError;

use super::{CreateOptimizationChatSessionInput, OptimizationSessionSummary};

mod request_preview;

pub use request_preview::preview_send_optimization_message_request;

pub async fn start_optimize_chat_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartOptimizeChatGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    start_optimize_chat_generation_with_output_limit(state, user_id, input, None).await
}

pub(crate) async fn start_optimize_chat_generation_with_output_limit(
    state: &AppState,
    user_id: i64,
    input: StartOptimizeChatGenerationInput,
    agent_max_output_tokens: Option<usize>,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .optimize_chat_generation_application()
        .start_chat_generation(user_id, &input)
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
                .optimize_chat_generation_application()
                .run_chat_generation(
                    prompt_renderer,
                    agent_max_output_tokens,
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
    let session = state
        .optimize_session_mutation_application()
        .create_chat_session(user_id, input)
        .await
        .map_err(|err| err.to_string())?;
    Ok(session)
}
