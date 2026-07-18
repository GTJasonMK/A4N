//! 灵感对话命令。
//!
//! Tauri 层只负责 command 签名和窗口事件；灵感对话业务逻辑
//! 统一在 `a4n_runtime::host_api::inspiration_records`。

use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::inspiration_records as inspiration_actions;
use inspiration_actions::{
    ConverseInput, ConverseStreamInput, MessageSummary, PreviewInspirationRequestInput,
};
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
pub async fn converse_inspiration(
    state: State<'_, AppState>,
    user_id: i64,
    input: ConverseInput,
) -> Result<String, String> {
    inspiration_actions::converse_inspiration(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
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

#[tauri::command]
pub async fn converse_inspiration_stream(
    state: State<'_, AppState>,
    user_id: i64,
    window: tauri::Window,
    input: ConverseStreamInput,
) -> Result<String, String> {
    let task_id =
        inspiration_actions::inspiration_stream_task_id(input.project_id, input.task_id.clone())
            .map_err(|e| e.to_string())?;
    let bridge = std::sync::Arc::new(crate::events::bridge::EventBridge::new(window));
    let bridge_for_chunk = bridge.clone();
    let task_id_for_chunk = task_id.clone();

    let result = inspiration_actions::converse_inspiration_stream(
        state.inner(),
        user_id,
        input,
        move |delta, idx| {
            let _ = bridge_for_chunk.emit_chunk(&task_id_for_chunk, delta, idx);
        },
    )
    .await;

    match result {
        Ok(full) => {
            let _ = bridge.emit_complete(&task_id, Some(full.clone()));
            Ok(full)
        }
        Err(err) => {
            let msg = err.to_string();
            let _ = bridge.emit_error(&task_id, "stream_error", &msg, false);
            Err(msg)
        }
    }
}
