use super::super::OptimizeSessionInput;
use crate::AppState;

pub async fn discard_pending_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    input: OptimizeSessionInput,
) -> Result<(), String> {
    state
        .orchestrator()
        .discard_pending_optimize_patch_sets_for_user_validated(user_id, input.session_id)
        .await
        .map_err(|error| error.to_string())
}
