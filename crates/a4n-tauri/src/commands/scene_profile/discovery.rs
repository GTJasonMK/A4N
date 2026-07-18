use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::host_api::scene_profile_sync::SceneProfileSyncProgress;
use serde_json::Value;
use tauri::{State, Window};

#[tauri::command]
pub async fn sync_scene_profiles(
    window: Window,
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    task_id: Option<String>,
) -> Result<Value, String> {
    let scope = a4n_runtime::host_api::scene_profile_sync::scene_profile_sync_task_scope(
        state.inner(),
        user_id,
        project_id,
    )
    .await?;
    let progress = WorkflowProgressTask::new(
        window,
        state.inner(),
        task_id,
        TaskType::LlmCall,
        Some(scope.project_id),
        scope.total,
        "正在准备场景档案同步",
    );
    let result = sync_scene_profiles_for_chapters(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
        Some(&progress),
    )
    .await;
    progress.finish_result(
        state.inner(),
        &result,
        "scene_profile_sync_failed",
        |value| {
            let profile_count = value
                .get("profile_ids")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            let occurrence_count = value
                .get("occurrence_count")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            format!("场景档案同步完成：关联 {profile_count} 个场景，记录 {occurrence_count} 次出场")
        },
    );
    result
}

pub(crate) async fn sync_scene_profiles_for_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&WorkflowProgressTask>,
) -> Result<Value, String> {
    if let Some(progress) = progress {
        let cancel_token = progress.cancel_token();
        let reporter = |current, total, message: &str| progress.progress(current, total, message);
        let runtime_progress = SceneProfileSyncProgress::new(Some(&reporter), Some(&cancel_token));
        a4n_runtime::host_api::scene_profile_sync::sync_scene_profiles_for_chapters(
            state,
            user_id,
            project_id,
            chapter_number,
            Some(&runtime_progress),
        )
        .await
    } else {
        a4n_runtime::host_api::scene_profile_sync::sync_scene_profiles_for_chapters(
            state,
            user_id,
            project_id,
            chapter_number,
            None,
        )
        .await
    }
}
