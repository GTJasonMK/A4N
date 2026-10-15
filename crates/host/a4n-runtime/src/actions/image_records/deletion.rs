use crate::{AppState, actions::app_error_message};

use super::DeleteImageInput;
use a4n_api_contract::image::ImageRecordDeleteResult;

pub async fn delete_image(
    state: &AppState,
    user_id: i64,
    input: DeleteImageInput,
) -> Result<ImageRecordDeleteResult, String> {
    state
        .image_record_cascade_mutation_application()
        .delete(user_id, input)
        .await
        .map_err(app_error_message)
}
