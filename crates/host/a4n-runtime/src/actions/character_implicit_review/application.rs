use super::{ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary};
use crate::AppState;

pub async fn apply_implicit_update_decision(
    state: &AppState,
    user_id: i64,
    input: ApplyImplicitUpdateDecisionInput,
) -> Result<ApplyImplicitUpdateDecisionResultSummary, String> {
    let applied = state
        .character_implicit_review_apply_application()
        .apply(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(applied)
}
