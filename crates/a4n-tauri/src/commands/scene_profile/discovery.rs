use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskKind;
use a4n_runtime::AppState;
use a4n_runtime::host_api::scene_profile_sync::SceneProfileSyncProgress;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn sync_scene_profiles(
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
        state.inner(),
        Some(user_id),
        task_id,
        TaskKind::LlmCall,
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
    progress.remove(state.inner());
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
        let runtime_progress = SceneProfileSyncProgress::new(
            Some(&reporter),
            Some(&cancel_token),
            progress.task_id(),
        );
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
