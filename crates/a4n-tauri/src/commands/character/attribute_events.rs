//! 角色属性事件命令。
//!
//! Tauri 层只保留 command 签名；属性事件写入、历史查询和 RAG 同步统一在
//! `a4n_runtime::host_api::character_records`。

use a4n_runtime::AppState;
use a4n_runtime::host_api::character_records as character_records_actions;
use character_records_actions::{
    AddProfileAttributeInput, AttributeEventSummary, AttributeHistoryInput,
    AttributeMutationResultSummary, ModifyProfileAttributeInput,
};
use tauri::State;

#[tauri::command]
pub async fn add_profile_attribute(
    state: State<'_, AppState>,
    user_id: i64,
    input: AddProfileAttributeInput,
) -> Result<AttributeMutationResultSummary, String> {
    character_records_actions::add_profile_attribute(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn modify_profile_attribute(
    state: State<'_, AppState>,
    user_id: i64,
    input: ModifyProfileAttributeInput,
) -> Result<AttributeMutationResultSummary, String> {
    character_records_actions::modify_profile_attribute(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn attribute_history(
    state: State<'_, AppState>,
    user_id: i64,
    input: AttributeHistoryInput,
) -> Result<Vec<AttributeEventSummary>, String> {
    character_records_actions::attribute_history(state.inner(), user_id, input).await
}
