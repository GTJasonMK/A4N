use super::super::OptimizeSessionInput;
use crate::AppState;

pub async fn discard_pending_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    input: OptimizeSessionInput,
) -> Result<(), String> {
    state
        .optimize_application()
        .discard_pending_patch_sets(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
