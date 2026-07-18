//! 物体档案命令。

pub(crate) mod image_generation;

use a4n_runtime::AppState;
use a4n_runtime::host_api::dependency_conflict::DownstreamConflictStrategy;
use a4n_runtime::host_api::object_profile_records as object_profile_records_actions;
use object_profile_records_actions::{
    ListObjectProfilesInput, ObjectProfileInput, ObjectProfileSummary,
};
use tauri::State;

#[tauri::command]
pub async fn list_object_profiles(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListObjectProfilesInput,
) -> Result<Vec<ObjectProfileSummary>, String> {
    object_profile_records_actions::list_object_profiles(state.inner(), user_id, input.project_id)
        .await
}

#[tauri::command]
pub async fn create_object_profile(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileSummary, String> {
    object_profile_records_actions::create_object_profile(state.inner(), user_id, project_id, input)
        .await
}

#[tauri::command]
pub async fn update_object_profile(
    state: State<'_, AppState>,
    user_id: i64,
    object_profile_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileSummary, String> {
    object_profile_records_actions::update_object_profile(
        state.inner(),
        user_id,
        object_profile_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn delete_object_profile(
    state: State<'_, AppState>,
    user_id: i64,
    object_profile_id: i64,
    downstream_strategy: Option<String>,
) -> Result<(), String> {
    let downstream_strategy =
        DownstreamConflictStrategy::from_optional_str(downstream_strategy.as_deref())?;
    object_profile_records_actions::delete_object_profile(
        state.inner(),
        user_id,
        object_profile_id,
        downstream_strategy,
    )
    .await
}
