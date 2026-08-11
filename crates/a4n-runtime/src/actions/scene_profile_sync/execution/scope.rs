use crate::state::AppState;
use a4n_orchestrator::scene_profile::SceneProfileSyncTaskScope;

pub async fn scene_profile_sync_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<SceneProfileSyncTaskScope, String> {
    state
        .orchestrator()
        .scene_profile_sync_task_scope_for_user_validated(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}
