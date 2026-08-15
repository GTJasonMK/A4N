use super::super::SyncCharacterProfileInput;
use crate::{AppState, actions::access};
use a4n_orchestrator::character::{
    CharacterProfileSyncApplicationInput, CharacterProfileSyncPreviewStep,
};

pub(crate) async fn character_sync_preview_step(
    state: &AppState,
    user_id: i64,
    input: SyncCharacterProfileInput,
) -> Result<Option<CharacterProfileSyncPreviewStep>, String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let prompt_renderer = state;
    state
        .orchestrator()
        .prepare_character_profile_sync_preview_validated(
            CharacterProfileSyncApplicationInput::preview_by_name(
                input.project_id,
                input.name,
                input.chapter_number,
            ),
            prompt_renderer,
        )
        .await
        .map_err(|error| error.to_string())
}
