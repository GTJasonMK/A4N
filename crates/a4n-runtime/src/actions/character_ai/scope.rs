use super::{CharacterAiTaskScope, SyncAllCharacterProfilesInput, SyncCharacterProfileInput};
use crate::{AppState, actions::access};
use a4n_orchestrator::character::{CHARACTER_PROFILE_SYNC_STEPS, CharacterDiscoveryInput};

pub async fn character_discovery_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<CharacterAiTaskScope, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    let scope = state
        .orchestrator()
        .character_discovery_task_scope_validated(&CharacterDiscoveryInput {
            project_id,
            chapter_number,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(CharacterAiTaskScope {
        project_id: scope.project_id,
        total: scope.total,
    })
}

pub async fn character_profile_sync_task_scope(
    state: &AppState,
    user_id: i64,
    input: &SyncCharacterProfileInput,
) -> Result<CharacterAiTaskScope, String> {
    character_ai_project_task_scope(
        state,
        user_id,
        input.project_id,
        CHARACTER_PROFILE_SYNC_STEPS,
    )
    .await
}

pub async fn character_profile_sync_all_task_scope(
    state: &AppState,
    user_id: i64,
    input: &SyncAllCharacterProfilesInput,
) -> Result<CharacterAiTaskScope, String> {
    character_ai_project_task_scope(state, user_id, input.project_id, 1).await
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
