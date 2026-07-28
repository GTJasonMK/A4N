use super::super::{
    ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse, contracts::orchestrator_patch,
};
use super::host_port::RuntimeOptimizePatchApplyHostPort;
use crate::AppState;
use a4n_orchestrator::optimize::ApplyOptimizePatchesApplicationInput;

pub async fn apply_optimization_patches(
    state: &AppState,
    user_id: i64,
    input: ApplyOptimizationPatchesInput,
) -> Result<ApplyOptimizationPatchesResponse, String> {
    let host_port = RuntimeOptimizePatchApplyHostPort::new(state);
    let patches = input
        .patches
        .iter()
        .map(orchestrator_patch)
        .collect::<Vec<_>>();
    state
        .optimize()
        .apply_optimize_patches_validated(ApplyOptimizePatchesApplicationInput {
            user_id,
            session_id: input.session_id,
            confirmed_apply: input.confirmed_apply,
            base_content_hash: &input.base_content_hash,
            patches: &patches,
            host_port: &host_port,
        })
        .await
        .map(|result| ApplyOptimizationPatchesResponse {
            session_id: result.session_id,
            chapter_id: result.chapter_id,
            applied_count: result.applied_count,
            word_count: result.word_count,
            content_hash: result.content_hash,
        })
        .map_err(|error| error.to_string())
}
