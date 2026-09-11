use super::super::GalgameMediaBindingSummary;
use super::ImportGalgameAudioInput;
use crate::AppState;

pub async fn import_galgame_audio(
    state: &AppState,
    user_id: i64,
    input: ImportGalgameAudioInput,
) -> Result<GalgameMediaBindingSummary, String> {
    state
        .galgame_record_application()
        .import_audio(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}
