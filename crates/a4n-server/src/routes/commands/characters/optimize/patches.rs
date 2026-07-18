use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, TaskType, command_input, finish_web_task, optimize_actions,
    optional_string_field, register_web_task, sanitize_optional_task_id, string_action_result,
    to_value, web_chunk_reporter, web_progress_reporter,
};
use super::task::{
    optimize_web_input_progress_task_command, optimize_web_input_stream_task_command,
    optimize_web_preview_command,
};

optimize_web_input_stream_task_command!(
    propose_optimization_patches,
    optimize_actions::ProposeOptimizationPatchesInput,
    optimize_actions::propose_optimization_patches_task_scope,
    "正在准备局部修复补丁",
    "局部补丁生成完成",
    optimize_actions::propose_optimization_patches,
);
optimize_web_preview_command!(
    preview_propose_optimization_patches_request,
    optimize_actions::ProposeOptimizationPatchesInput,
    optimize_actions::preview_propose_optimization_patches_request,
);
optimize_web_input_progress_task_command!(
    apply_optimization_patches,
    optimize_actions::ApplyOptimizationPatchesInput,
    optimize_actions::apply_optimization_patches_task_scope,
    "正在应用局部补丁",
    "局部补丁应用完成",
    optimize_actions::apply_optimization_patches,
);
