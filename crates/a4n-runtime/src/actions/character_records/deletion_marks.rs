use super::access::require_profile_for_user;
use super::contracts::{
    DeletionMarkSummary, ExecuteAttributeDeletionInput, ExecuteAttributeDeletionResultSummary,
    ListDeletionMarksInput, RequestAttributeDeletionInput, RequestAttributeDeletionResultSummary,
    ResetDeletionMarksInput, ResetDeletionMarksResultSummary,
};
use super::projections::{attribute_event_summary, deletion_mark_summary};
use crate::actions::app_error_message;
use crate::state::AppState;
use a4n_orchestrator::character::{CharacterAttributeDeletionRequestInput, DELETION_THRESHOLD};

pub async fn list_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ListDeletionMarksInput,
) -> Result<Vec<DeletionMarkSummary>, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let marks = state
        .character()
        .deletion_marks_by_profile_validated(
            input.profile_id,
            input.category.as_deref(),
            input.status.as_deref(),
            DELETION_THRESHOLD,
        )
        .await
        .map_err(app_error_message)?;
    Ok(marks.iter().map(deletion_mark_summary).collect())
}

pub async fn request_attribute_deletion(
    state: &AppState,
    user_id: i64,
    input: RequestAttributeDeletionInput,
) -> Result<RequestAttributeDeletionResultSummary, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let mark = state
        .character()
        .request_attribute_deletion_validated(CharacterAttributeDeletionRequestInput {
            profile_id: input.profile_id,
            category: input.category,
            key: input.key,
            chapter_number: input.chapter_number,
            reason: input.reason,
            evidence: input.evidence,
        })
        .await
        .map_err(app_error_message)?;
    Ok(RequestAttributeDeletionResultSummary {
        ready_for_deletion: !mark.is_executed && mark.consecutive_count >= DELETION_THRESHOLD,
        threshold: DELETION_THRESHOLD,
        mark: deletion_mark_summary(&mark),
    })
}

pub async fn execute_attribute_deletion(
    state: &AppState,
    user_id: i64,
    input: ExecuteAttributeDeletionInput,
) -> Result<ExecuteAttributeDeletionResultSummary, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let result = state
        .character()
        .execute_ready_attribute_deletion_validated(
            input.profile_id,
            input.category,
            input.key,
            input.chapter_number,
            input.cause,
            input.evidence,
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(ExecuteAttributeDeletionResultSummary {
        event: attribute_event_summary(&result.event),
        consecutive_count: result.consecutive_count,
        threshold: DELETION_THRESHOLD,
    })
}

pub async fn reset_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ResetDeletionMarksInput,
) -> Result<ResetDeletionMarksResultSummary, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let deleted_marks = state
        .character()
        .reset_deletion_marks_validated(input.profile_id, &input.category, &input.key)
        .await
        .map_err(app_error_message)?;
    Ok(ResetDeletionMarksResultSummary { deleted_marks })
}
