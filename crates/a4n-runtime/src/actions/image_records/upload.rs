mod file;
mod record;

use crate::AppState;
use a4n_api_contract::image::GeneratedImageSummary;

use super::UploadImageInput;

pub async fn upload_image(
    state: &AppState,
    user_id: i64,
    input: UploadImageInput,
) -> Result<GeneratedImageSummary, String> {
    import_image_record(state, user_id, input).await
}

pub(crate) async fn import_image_record(
    state: &AppState,
    user_id: i64,
    input: UploadImageInput,
) -> Result<GeneratedImageSummary, String> {
    let prepared = state
        .image_import_application()
        .prepare_imported_upload(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    record::persist_imported_image(state, user_id, prepared).await
}
