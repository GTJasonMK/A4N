use super::SceneProfileSummary;
use crate::AppState;

pub async fn list_scene_profiles(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<SceneProfileSummary>, String> {
    state
        .scene_profile_read_application()
        .list(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}
