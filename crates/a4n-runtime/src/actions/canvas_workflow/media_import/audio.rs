use super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use super::canvas_media_asset_summary;
use crate::AppState;
use a4n_orchestrator::canvas::CanvasAudioImportHostInput;
use a4n_orchestrator::media::MEDIA_KIND_AUDIO;

pub(super) async fn import_canvas_audio(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let asset = state
        .orchestrator()
        .import_canvas_audio_validated(user_id, CanvasAudioImportHostInput {
            project_id: input.project_id,
            file_path: input.file_path,
            file_name: input.file_name,
            mime_type: input.mime_type,
            data_base64: input.data_base64,
        })
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(ImportedCanvasMedia {
        media_kind: MEDIA_KIND_AUDIO.to_string(),
        image: None,
        video: None,
        asset: canvas_media_asset_summary(asset),
    })
}
