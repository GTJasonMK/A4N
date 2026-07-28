use serde_json::{Value, json};

use super::super::super::{ApiError, ServerState, command_input, optimize_actions, to_value};
use super::adapters::{optimize_web_input_command, optimize_web_preview_command};
optimize_web_preview_command!(
    preview_propose_optimization_patches_request,
    optimize_actions::ProposeOptimizationPatchesInput,
    optimize_actions::preview_propose_optimization_patches_request,
);
optimize_web_input_command!(
    apply_optimization_patches,
    optimize_actions::ApplyOptimizationPatchesInput,
    optimize_actions::apply_optimization_patches,
);
