//! 角色档案命令。
//!
//! Tauri 层只保留 command 签名；角色档案业务逻辑统一在
//! `a4n_runtime::host_api::character_records`。

use a4n_runtime::AppState;
use a4n_runtime::host_api::character_records as character_records_actions;
use character_records_actions::{
    CreateProfileInput, DeleteProfileResultSummary, ListProfilesInput, ProtagonistProfileSummary,
};
use tauri::State;

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    user_id: i64,
    input: CreateProfileInput,
) -> Result<ProtagonistProfileSummary, String> {
    character_records_actions::create_profile(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn delete_profile(
    state: State<'_, AppState>,
    user_id: i64,
    profile_id: i64,
) -> Result<DeleteProfileResultSummary, String> {
    character_records_actions::delete_profile(state.inner(), user_id, profile_id).await
}

#[tauri::command]
pub async fn list_profiles(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListProfilesInput,
) -> Result<Vec<ProtagonistProfileSummary>, String> {
    character_records_actions::list_profiles(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn get_character_arc_status(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    name: String,
) -> Result<Option<serde_json::Value>, String> {
    character_records_actions::get_character_arc_status(state.inner(), user_id, project_id, name)
        .await
}

#[tauri::command]
pub async fn get_character_voice_fingerprint(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    name: String,
) -> Result<Option<serde_json::Value>, String> {
    character_records_actions::get_character_voice_fingerprint(
        state.inner(),
        user_id,
        project_id,
        name,
    )
    .await
}
