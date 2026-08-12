use super::{ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary};
use crate::{AppState, actions::character_records::deletion_mark_summary};
use a4n_domain::character::AppliedImplicitUpdateAction;
use a4n_orchestrator::character::AppliedCharacterImplicitDecisionAction;

pub async fn apply_implicit_update_decision(
    state: &AppState,
    user_id: i64,
    input: ApplyImplicitUpdateDecisionInput,
) -> Result<ApplyImplicitUpdateDecisionResultSummary, String> {
    let applied = state
        .orchestrator()
        .apply_implicit_update_decision_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ApplyImplicitUpdateDecisionResultSummary {
        action: match applied.action {
            AppliedCharacterImplicitDecisionAction::Keep => AppliedImplicitUpdateAction::Keep,
            AppliedCharacterImplicitDecisionAction::Modify => AppliedImplicitUpdateAction::Modify,
            AppliedCharacterImplicitDecisionAction::MarkDeletion => {
                AppliedImplicitUpdateAction::MarkDeletion
            }
        },
        deletion_mark: applied
            .deletion_mark
            .as_ref()
            .map(deletion_mark_summary)
            .transpose()?,
    })
}
