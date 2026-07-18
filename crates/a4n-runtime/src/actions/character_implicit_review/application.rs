use super::{
    ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary,
    context::verify_profile_ownership,
    projections::{to_deletion_mark_summary, to_event_summary},
};
use crate::AppState;
use a4n_orchestrator::character::{
    AppliedCharacterImplicitDecisionAction, CharacterImplicitReviewDecisionRecordInput,
    DELETION_THRESHOLD,
};

pub async fn apply_implicit_update_decision(
    state: &AppState,
    user_id: i64,
    input: ApplyImplicitUpdateDecisionInput,
) -> Result<ApplyImplicitUpdateDecisionResultSummary, String> {
    let profile_id = input.decision.profile_id;
    verify_profile_ownership(state, user_id, profile_id).await?;
    let applied = state
        .orchestrator
        .apply_implicit_review_decision_record(CharacterImplicitReviewDecisionRecordInput {
            profile_id,
            chapter_number: input.chapter_number,
            attribute_key: input.decision.attribute_key.clone(),
            current_value: input.decision.current_value.clone(),
            stats_attribute_key: input.decision.stats.attribute_key.clone(),
            stats_current_value: input.decision.stats.current_value.clone(),
            decision: input.decision.decision.clone(),
            suggested_new_value: input.decision.suggested_new_value.clone(),
            reasoning: input.decision.reasoning.clone(),
            evidence_summary: input.decision.evidence_summary.clone(),
        })
        .await
        .map_err(|error| error.to_string())?;

    let ready_for_deletion = applied
        .deletion_mark
        .as_ref()
        .is_some_and(|mark| !mark.is_executed && mark.consecutive_count >= DELETION_THRESHOLD);

    Ok(ApplyImplicitUpdateDecisionResultSummary {
        action: match applied.action {
            AppliedCharacterImplicitDecisionAction::Keep => "keep",
            AppliedCharacterImplicitDecisionAction::Modify => "modify",
            AppliedCharacterImplicitDecisionAction::MarkDeletion => "mark_deletion",
        }
        .to_string(),
        event: applied.event.as_ref().map(to_event_summary),
        deletion_mark: applied.deletion_mark.as_ref().map(to_deletion_mark_summary),
        ready_for_deletion,
        threshold: DELETION_THRESHOLD,
    })
}
