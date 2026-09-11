use crate::{
    AppState,
    actions::{app_error_message, image_generation::GeneratedImageSummary},
};

use super::UpdateGeneratedImageInput;

pub async fn update_image(
    state: &AppState,
    user_id: i64,
    input: UpdateGeneratedImageInput,
) -> Result<GeneratedImageSummary, String> {
    state
        .image_record_application()
        .update(user_id, input)
        .await
        .map_err(app_error_message)
}
