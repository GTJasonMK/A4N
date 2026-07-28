use super::access::require_profile_for_user;
use super::contracts::{BehaviorRecordSummary, ListBehaviorsInput};
use super::projections::behavior_record_summary;
use crate::actions::app_error_message;
use crate::state::AppState;

pub async fn list_behaviors(
    state: &AppState,
    user_id: i64,
    input: ListBehaviorsInput,
) -> Result<Vec<BehaviorRecordSummary>, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let records = state
        .character()
        .behavior_records_by_profile(
            input.profile_id,
            input.chapter_from,
            input.chapter_to,
            input.limit,
        )
        .await
        .map_err(app_error_message)?;
    Ok(records.iter().map(behavior_record_summary).collect())
}
