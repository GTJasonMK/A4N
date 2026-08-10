use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, HostErrorResponse},
    optimize::StartOptimizeChatGenerationInput,
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::optimize::{
    self as optimize_actions, CreateOptimizationChatSessionInput, OptimizationSessionSummary,
    SendOptimizationMessageInput,
};
use tauri::State;

use super::adapters::{optimize_input_command, optimize_preview_command};

optimize_input_command!(
    create_optimization_chat_session,
    CreateOptimizationChatSessionInput,
    OptimizationSessionSummary,
    optimize_actions::create_optimization_chat_session,
);

#[tauri::command]
pub async fn start_optimize_chat_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartOptimizeChatGenerationInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    optimize_actions::start_optimize_chat_generation(state.inner(), user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}

optimize_preview_command!(
    preview_send_optimization_message_request,
    SendOptimizationMessageInput,
    optimize_actions::preview_send_optimization_message_request,
);
