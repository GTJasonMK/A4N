use super::super::super::SyncCharacterProfileInput;
use crate::AppState;
use a4n_character_host_contract::CharacterProfileSyncPreviewStep;

pub(crate) async fn character_sync_preview_step(
    state: &AppState,
    user_id: i64,
    input: SyncCharacterProfileInput,
) -> Result<Option<CharacterProfileSyncPreviewStep>, String> {
    let prompt_renderer = state;
    state
        .character_profile_sync_application()
        .preview_profile_sync(user_id, input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())
}
