use super::access::{
    require_profile_for_user, required_positive_chapter_number, required_trimmed_string,
};
use super::contracts::{
    DeletionMarkSummary, ExecuteAttributeDeletionInput, ExecuteAttributeDeletionResultSummary,
    ListDeletionMarksInput, RequestAttributeDeletionInput, RequestAttributeDeletionResultSummary,
    ResetDeletionMarksInput, ResetDeletionMarksResultSummary,
};
use super::projections::{attribute_event_summary, deletion_mark_summary};
use crate::actions::app_error_message;
use crate::state::AppState;
use a4n_orchestrator::character::{
    CharacterAttributeDeletionExecutionInput, CharacterAttributeDeletionRequestInput,
    DELETION_THRESHOLD, DeletionMarkListStatus,
};

pub async fn list_deletion_marks(
    state: &AppState,
    user_id: i64,
    input: ListDeletionMarksInput,
) -> Result<Vec<DeletionMarkSummary>, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let category = if let Some(category) = input
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !matches!(category, "explicit" | "implicit" | "social") {
            return Err("删除标记 category 必须是 explicit/implicit/social".to_string());
        }
        Some(category)
    } else {
        None
    };

    let status = match input
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None | Some("all") => DeletionMarkListStatus::All,
        Some("pending") => DeletionMarkListStatus::Pending,
        Some("ready") => DeletionMarkListStatus::Ready,
        Some("executed") => DeletionMarkListStatus::Executed,
        Some(_) => return Err("删除标记 status 必须是 all/pending/ready/executed".to_string()),
    };

    let marks = state
        .orchestrator
        .deletion_marks_by_profile(input.profile_id, category, status, DELETION_THRESHOLD)
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
        .orchestrator
        .request_attribute_deletion(CharacterAttributeDeletionRequestInput {
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
    let chapter_number = required_positive_chapter_number(input.chapter_number, "执行属性删除")?;
    let cause = required_trimmed_string(input.cause, "删除原因")?;
    let result = state
        .orchestrator
        .execute_ready_attribute_deletion(CharacterAttributeDeletionExecutionInput {
            profile_id: input.profile_id,
            category: input.category,
            key: input.key,
            event_cause: cause,
            evidence: input.evidence,
            chapter_number,
        })
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
        .orchestrator
        .reset_deletion_marks(input.profile_id, &input.category, &input.key)
        .await
        .map_err(app_error_message)?;
    Ok(ResetDeletionMarksResultSummary { deleted_marks })
}
