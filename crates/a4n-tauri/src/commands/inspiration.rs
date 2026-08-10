//! 灵感对话命令。
//!
//! Tauri 层只负责 command 签名和窗口事件；灵感对话业务逻辑
//! 统一在 `a4n_runtime::host_api::inspiration_records`。

use crate::commands::request_preview::ModelRequestPreview;
use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, HostErrorResponse},
    inspiration::StartInspirationGenerationInput,
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::inspiration_records as inspiration_actions;
use inspiration_actions::{MessageSummary, PreviewInspirationRequestInput};
use tauri::State;

#[tauri::command]
pub async fn get_inspiration_history(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    limit: u64,
) -> Result<Vec<MessageSummary>, String> {
    inspiration_actions::get_inspiration_history(state.inner(), user_id, project_id, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_inspiration_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartInspirationGenerationInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    inspiration_actions::start_inspiration_generation(state.inner(), user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}

#[tauri::command]
pub async fn preview_inspiration_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: PreviewInspirationRequestInput,
) -> Result<ModelRequestPreview, String> {
    inspiration_actions::preview_inspiration_request(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}
