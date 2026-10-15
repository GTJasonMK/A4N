use super::{ObjectProfileListItem, ObjectProfileSummary};
use crate::AppState;

pub async fn list_object_profiles(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ObjectProfileSummary>, String> {
    state
        .object_profile_read_application()
        .list(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn list_object_profile_index(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ObjectProfileListItem>, String> {
    state
        .object_profile_read_application()
        .list_index(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}
