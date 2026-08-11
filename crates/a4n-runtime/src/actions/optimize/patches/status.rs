use crate::AppState;

pub async fn reject_pending_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<(), String> {
    state
        .orchestrator()
        .reject_pending_optimize_patch_sets_for_user_validated(user_id, session_id)
        .await
        .map_err(|error| error.to_string())
}
