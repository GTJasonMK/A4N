use a4n_runtime::host_api::optimize::{
    self as optimize_actions, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
    ProposeOptimizationPatchesInput, ProposeOptimizationPatchesResponse,
};

use super::adapters::optimize_preview_command;
use super::task::{optimize_input_progress_task_command, optimize_input_stream_task_command};

optimize_input_stream_task_command!(
    propose_optimization_patches,
    ProposeOptimizationPatchesInput,
    ProposeOptimizationPatchesResponse,
    optimize_actions::propose_optimization_patches_task_scope,
    "正在准备局部修复补丁",
    "局部补丁生成已停止",
    "optimize_patch_failed",
    |response| format!("已生成 {} 个局部补丁", response.patches.len()),
    optimize_actions::propose_optimization_patches,
);
optimize_preview_command!(
    preview_propose_optimization_patches_request,
    ProposeOptimizationPatchesInput,
    optimize_actions::preview_propose_optimization_patches_request,
);
optimize_input_progress_task_command!(
    apply_optimization_patches,
    ApplyOptimizationPatchesInput,
    ApplyOptimizationPatchesResponse,
    optimize_actions::apply_optimization_patches_task_scope,
    "正在应用局部补丁",
    "应用局部补丁已停止",
    "optimize_patch_apply_failed",
    |response| format!(
        "已应用 {} 个局部补丁，字数 {}",
        response.applied_count, response.word_count
    ),
    optimize_actions::apply_optimization_patches,
);
