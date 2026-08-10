use super::super::GalgameMediaBindingSummary;
use super::ImportGalgameAudioInput;
use crate::AppState;

pub async fn import_galgame_audio(
    state: &AppState,
    user_id: i64,
    input: ImportGalgameAudioInput,
) -> Result<GalgameMediaBindingSummary, String> {
    state
        .galgame()
        .import_galgame_audio_for_user_validated(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}
