use super::super::{
    ApplyOptimizationPatchesInput, OptimizationTaskScope, ProposeOptimizationPatchesInput,
    access::optimize_session_task_scope,
};
use crate::AppState;

pub async fn propose_optimization_patches_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ProposeOptimizationPatchesInput,
) -> Result<OptimizationTaskScope, String> {
    optimize_session_task_scope(state, user_id, input.session_id, 4).await
}

pub async fn apply_optimization_patches_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ApplyOptimizationPatchesInput,
) -> Result<OptimizationTaskScope, String> {
    optimize_session_task_scope(state, user_id, input.session_id, 3).await
}
