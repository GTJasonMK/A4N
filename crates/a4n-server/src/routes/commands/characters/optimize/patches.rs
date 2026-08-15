use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, optimize_actions, strict_runtime_command_input, to_value,
};
use super::adapters::optimize_web_runtime_input_command;
optimize_web_runtime_input_command!(
    preview_propose_optimization_patches_request,
    optimize_actions::ProposeOptimizationPatchesInput,
    optimize_actions::preview_propose_optimization_patches_request,
);
optimize_web_runtime_input_command!(
    apply_optimization_patches,
    optimize_actions::ApplyOptimizationPatchesInput,
    optimize_actions::apply_optimization_patches,
);
