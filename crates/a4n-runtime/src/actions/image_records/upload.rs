mod file;
mod record;

use crate::{AppState, actions::image_generation::GeneratedImageSummary};

use super::UploadImageInput;

pub async fn upload_image(
    state: &AppState,
    user_id: i64,
    input: UploadImageInput,
) -> Result<GeneratedImageSummary, String> {
    let prepared = state
        .orchestrator()
        .prepare_imported_image_upload_for_user_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    record::persist_imported_image(state, user_id, prepared).await
}
