use super::access::require_profile_for_user;
use super::projections::deletion_mark_summary;
use crate::actions::app_error_message;
use crate::state::AppState;
use a4n_api_contract::character::{
    DeletionMarkSummary, ExecuteAttributeDeletionInput, ListDeletionMarksInput,
    RequestAttributeDeletionInput, ResetDeletionMarksInput,
};
use a4n_orchestrator::character::{
    CharacterAttributeDeletionExecutionInput, CharacterAttributeDeletionRequestInput,
};

pub async fn list_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ListDeletionMarksInput,
) -> Result<Vec<DeletionMarkSummary>, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let marks = state
        .orchestrator()
        .deletion_marks_by_profile_validated(input.profile_id, input.category, input.status)
        .await
        .map_err(app_error_message)?;
    marks.iter().map(deletion_mark_summary).collect()
}

pub async fn request_attribute_deletion(
    state: &AppState,
    user_id: i64,
    input: RequestAttributeDeletionInput,
) -> Result<DeletionMarkSummary, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let mark = state
        .orchestrator()
        .request_attribute_deletion_validated(CharacterAttributeDeletionRequestInput {
            profile_id: input.profile_id,
            category: input.category.as_str().to_string(),
            key: input.key,
            chapter_number: input.chapter_number,
            reason: input.reason,
            evidence: input.evidence,
        })
        .await
        .map_err(app_error_message)?;
    deletion_mark_summary(&mark)
}

pub async fn execute_attribute_deletion(
    state: &AppState,
    user_id: i64,
    input: ExecuteAttributeDeletionInput,
) -> Result<(), String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    state
        .orchestrator()
        .execute_ready_attribute_deletion_validated(CharacterAttributeDeletionExecutionInput {
            profile_id: input.profile_id,
            category: input.category.as_str().to_string(),
            key: input.key,
            event_cause: input.cause,
            evidence: input.evidence,
            chapter_number: input.chapter_number,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub async fn reset_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ResetDeletionMarksInput,
) -> Result<u64, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let deleted_marks = state
        .orchestrator()
        .reset_deletion_marks_validated(input.profile_id, input.category, &input.key)
        .await
        .map_err(app_error_message)?;
    Ok(deleted_marks)
}
