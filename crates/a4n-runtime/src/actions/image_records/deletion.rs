use crate::{AppState, actions::app_error_message};

pub async fn delete_image(state: &AppState, user_id: i64, image_id: i64) -> Result<(), String> {
    state
        .orchestrator()
        .delete_generated_image_for_user_validated(user_id, image_id)
        .await
        .map_err(app_error_message)
}
