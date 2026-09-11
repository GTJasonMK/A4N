use crate::actions::app_error_message;
use crate::state::AppState;
use a4n_api_contract::character::{BehaviorRecordSummary, ListBehaviorsInput};

pub async fn list_behaviors(
    state: &AppState,
    user_id: i64,
    input: ListBehaviorsInput,
) -> Result<Vec<BehaviorRecordSummary>, String> {
    state
        .character_profile_read_application()
        .behavior_history(user_id, input)
        .await
        .map_err(app_error_message)
}
