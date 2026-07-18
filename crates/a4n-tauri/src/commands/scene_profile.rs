//! 场景档案命令。

pub(crate) mod discovery;
pub(crate) mod image_generation;

use a4n_runtime::AppState;
use a4n_runtime::host_api::dependency_conflict::DownstreamConflictStrategy;
use a4n_runtime::host_api::scene_profile_records as scene_profile_records_actions;
use scene_profile_records_actions::{
    ListSceneProfilesInput, SceneProfileInput, SceneProfileSummary,
};
use tauri::State;

#[tauri::command]
pub async fn list_scene_profiles(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListSceneProfilesInput,
) -> Result<Vec<SceneProfileSummary>, String> {
    scene_profile_records_actions::list_scene_profiles(state.inner(), user_id, input.project_id)
        .await
}

#[tauri::command]
pub async fn create_scene_profile(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: SceneProfileInput,
) -> Result<SceneProfileSummary, String> {
    scene_profile_records_actions::create_scene_profile(state.inner(), user_id, project_id, input)
        .await
}

#[tauri::command]
pub async fn update_scene_profile(
    state: State<'_, AppState>,
    user_id: i64,
    scene_profile_id: i64,
    input: SceneProfileInput,
) -> Result<SceneProfileSummary, String> {
    scene_profile_records_actions::update_scene_profile(
        state.inner(),
        user_id,
        scene_profile_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn delete_scene_profile(
    state: State<'_, AppState>,
    user_id: i64,
    scene_profile_id: i64,
    downstream_strategy: Option<String>,
) -> Result<(), String> {
    let downstream_strategy =
        DownstreamConflictStrategy::from_optional_str(downstream_strategy.as_deref())?;
    scene_profile_records_actions::delete_scene_profile(
        state.inner(),
        user_id,
        scene_profile_id,
        downstream_strategy,
    )
    .await
}
