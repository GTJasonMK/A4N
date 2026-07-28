use super::{ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary};
use crate::AppState;

pub async fn apply_implicit_update_decision(
    state: &AppState,
    user_id: i64,
    input: ApplyImplicitUpdateDecisionInput,
) -> Result<ApplyImplicitUpdateDecisionResultSummary, String> {
    state
        .character()
        .apply_implicit_update_decision_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
