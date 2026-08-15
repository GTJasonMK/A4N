use super::super::{ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse};
use super::host_port::RuntimeOptimizePatchApplyHostPort;
use crate::AppState;
use a4n_orchestrator::optimize::ApplyOptimizePatchesApplicationInput;

pub async fn apply_optimization_patches(
    state: &AppState,
    user_id: i64,
    input: ApplyOptimizationPatchesInput,
) -> Result<ApplyOptimizationPatchesResponse, String> {
    let host_port = RuntimeOptimizePatchApplyHostPort::new(state);
    state
        .orchestrator()
        .apply_optimize_patches_validated(ApplyOptimizePatchesApplicationInput {
            user_id,
            session_id: input.session_id,
            base_content_hash: &input.base_content_hash,
            patches: &input.patches,
            host_port: &host_port,
        })
        .await
        .map(|result| ApplyOptimizationPatchesResponse {
            chapter_id: result.chapter_id,
            applied_count: result.applied_count,
            word_count: result.word_count,
        })
        .map_err(|error| error.to_string())
}
