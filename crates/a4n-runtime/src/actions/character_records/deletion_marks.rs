use crate::actions::app_error_message;
use crate::state::AppState;
use a4n_api_contract::character::{
    DeletionMarkSummary, ExecuteAttributeDeletionInput, ListDeletionMarksInput,
    RequestAttributeDeletionInput, ResetDeletionMarksInput,
};

pub async fn list_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ListDeletionMarksInput,
) -> Result<Vec<DeletionMarkSummary>, String> {
    state
        .character_timeline_read_application()
        .deletion_marks(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn request_attribute_deletion(
    state: &AppState,
    user_id: i64,
    input: RequestAttributeDeletionInput,
) -> Result<DeletionMarkSummary, String> {
    state
        .character_profile_attribute_deletion_application()
        .request(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn execute_attribute_deletion(
    state: &AppState,
    user_id: i64,
    input: ExecuteAttributeDeletionInput,
) -> Result<(), String> {
    state
        .character_profile_attribute_deletion_application()
        .execute(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn reset_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ResetDeletionMarksInput,
) -> Result<u64, String> {
    state
        .character_profile_attribute_deletion_application()
        .reset(user_id, input)
        .await
        .map_err(app_error_message)
}
