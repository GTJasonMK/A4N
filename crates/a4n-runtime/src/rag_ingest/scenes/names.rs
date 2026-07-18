use crate::state::AppState;
use std::collections::HashMap;

pub(super) async fn scene_profile_names(
    state: &AppState,
    project_id: i64,
) -> Result<HashMap<i64, String>, String> {
    let profiles = state
        .orchestrator
        .scene_profiles_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(profiles
        .into_iter()
        .map(|profile| (profile.id, profile.name))
        .collect())
}
