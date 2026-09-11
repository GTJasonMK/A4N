use crate::{AppState, actions::app_error_message};

use super::DeleteImageInput;

pub async fn delete_image(
    state: &AppState,
    user_id: i64,
    input: DeleteImageInput,
) -> Result<(), String> {
    state
        .image_record_application()
        .delete(user_id, input)
        .await
        .map_err(app_error_message)
}
