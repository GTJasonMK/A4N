use crate::{AppState, actions::app_error_message};

use super::DeleteImageInput;

pub async fn delete_image(
    state: &AppState,
    user_id: i64,
    input: DeleteImageInput,
) -> Result<(), String> {
    state
        .orchestrator()
        .delete_generated_image_for_user_validated(user_id, input.image_id)
        .await
        .map_err(app_error_message)
}
