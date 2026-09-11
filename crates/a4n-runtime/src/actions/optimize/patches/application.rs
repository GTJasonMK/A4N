use super::super::{ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse};
use super::host_port::RuntimeOptimizePatchApplyHostPort;
use crate::AppState;

pub async fn apply_optimization_patches(
    state: &AppState,
    user_id: i64,
    input: ApplyOptimizationPatchesInput,
) -> Result<ApplyOptimizationPatchesResponse, String> {
    let host_port = RuntimeOptimizePatchApplyHostPort::new(state);
    state
        .optimize_application()
        .apply_patches(user_id, input, &host_port)
        .await
        .map_err(|error| error.to_string())
}
