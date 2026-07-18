//! 角色属性删除标记命令。
//!
//! Tauri 层只保留 command 签名；删除保护、执行删除和事件 RAG 同步统一在
//! `a4n_runtime::host_api::character_records`。

use a4n_runtime::AppState;
use a4n_runtime::host_api::character_records as character_records_actions;
use character_records_actions::{
    DeletionMarkSummary, ExecuteAttributeDeletionInput, ExecuteAttributeDeletionResultSummary,
    ListDeletionMarksInput, RequestAttributeDeletionInput, RequestAttributeDeletionResultSummary,
    ResetDeletionMarksInput, ResetDeletionMarksResultSummary,
};
use tauri::State;

#[tauri::command]
pub async fn list_deletion_marks(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListDeletionMarksInput,
) -> Result<Vec<DeletionMarkSummary>, String> {
    character_records_actions::list_deletion_marks(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn request_attribute_deletion(
    state: State<'_, AppState>,
    user_id: i64,
    input: RequestAttributeDeletionInput,
) -> Result<RequestAttributeDeletionResultSummary, String> {
    character_records_actions::request_attribute_deletion(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn execute_attribute_deletion(
    state: State<'_, AppState>,
    user_id: i64,
    input: ExecuteAttributeDeletionInput,
) -> Result<ExecuteAttributeDeletionResultSummary, String> {
    character_records_actions::execute_attribute_deletion(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn reset_deletion_marks(
    state: State<'_, AppState>,
    user_id: i64,
    input: ResetDeletionMarksInput,
) -> Result<ResetDeletionMarksResultSummary, String> {
    character_records_actions::reset_deletion_marks(state.inner(), user_id, input).await
}
