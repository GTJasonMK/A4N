use super::{CHARACTER_PROFILE_SYNC_STEPS, CharacterAiTaskScope};
use crate::{AppState, actions::access};

pub async fn character_discovery_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<CharacterAiTaskScope, String> {
    character_ai_project_task_scope(state, user_id, project_id, 100).await
}

pub async fn character_profile_sync_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<CharacterAiTaskScope, String> {
    character_ai_project_task_scope(state, user_id, project_id, CHARACTER_PROFILE_SYNC_STEPS).await
}

pub async fn character_profile_sync_all_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<CharacterAiTaskScope, String> {
    character_ai_project_task_scope(state, user_id, project_id, 1).await
}

async fn character_ai_project_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    total: u32,
) -> Result<CharacterAiTaskScope, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(CharacterAiTaskScope { project_id, total })
}
