use super::super::progress::SCENE_PROFILE_SYNC_PROGRESS_TOTAL;
use super::super::types::SceneProfileSyncTaskScope;
use crate::actions::access;
use crate::state::AppState;

pub async fn scene_profile_sync_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<SceneProfileSyncTaskScope, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(SceneProfileSyncTaskScope {
        project_id,
        total: SCENE_PROFILE_SYNC_PROGRESS_TOTAL,
    })
}
