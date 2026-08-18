mod file;
mod record;

use crate::{AppState, actions::image_generation::GeneratedImageSummary};
use a4n_core::models::ai::generated_image;

use super::UploadImageInput;

pub async fn upload_image(
    state: &AppState,
    user_id: i64,
    input: UploadImageInput,
) -> Result<GeneratedImageSummary, String> {
    let record = import_image_record(state, user_id, input).await?;
    Ok(a4n_orchestrator::media::generated_image_summary(&record))
}

pub(crate) async fn import_image_record(
    state: &AppState,
    user_id: i64,
    input: UploadImageInput,
) -> Result<generated_image::Model, String> {
    let prepared = state
        .orchestrator()
        .prepare_imported_image_upload_for_user_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    record::persist_imported_image(state, user_id, prepared).await
}
