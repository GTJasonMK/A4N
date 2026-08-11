mod file;
mod record;

use crate::{AppState, actions::image_generation::GeneratedImageSummary};
use a4n_orchestrator::media::ImportedImageUploadInput;

use super::UploadImageInput;

pub async fn upload_image(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UploadImageInput,
) -> Result<GeneratedImageSummary, String> {
    let prepared = state
        .orchestrator()
        .prepare_imported_image_upload_for_user_validated(
            user_id,
            project_id,
            ImportedImageUploadInput {
                file_name: input.file_name,
                mime_type: input.mime_type,
                data_base64: input.data_base64,
                image_type: input.image_type,
                prompt: input.prompt,
                chapter_id: input.chapter_id,
                panel_id: input.panel_id,
                metadata_json: input.metadata_json.map(serde_json::Value::Object),
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    record::persist_imported_image(state, user_id, prepared).await
}
