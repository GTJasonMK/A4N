use super::super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::AppState;
use a4n_orchestrator::canvas::CanvasAudioImportHostInput;

pub(in crate::actions::canvas_workflow::media_import) async fn import_canvas_audio(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let asset = state
        .orchestrator()
        .import_canvas_audio_validated(CanvasAudioImportHostInput {
            user_id,
            project_id: input.project_id,
            file_path: input.file_path,
            file_name: input.file_name,
            mime_type: input.mime_type,
            data_base64: input.data_base64,
        })
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(ImportedCanvasMedia {
        media_kind: "audio".to_string(),
        image: None,
        video: None,
        asset: serde_json::to_value(asset)
            .map_err(|error| format!("序列化画布音频资产失败: {error}"))?,
    })
}
