use super::super::{DiscardOptimizationPatchSetInput, OptimizeSessionInput};
use crate::AppState;

pub async fn discard_pending_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    input: OptimizeSessionInput,
) -> Result<(), String> {
    state
        .optimize_session_mutation_application()
        .discard_pending_patch_sets(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn discard_optimization_patch_set(
    state: &AppState,
    user_id: i64,
    input: DiscardOptimizationPatchSetInput,
) -> Result<(), String> {
    state
        .optimize_session_mutation_application()
        .discard_patch_set(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
