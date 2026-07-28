use a4n_core::models::{canvas::canvas_node, generation::workflow_checkpoint};
use serde_json::Value;

use crate::canvas::canvas_node_is_operation;

pub(crate) fn source_operation_checkpoint_output<'a>(
    checkpoints: Option<&'a [workflow_checkpoint::Model]>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<&'a Value>, String> {
    if !canvas_node_is_operation(&node.node_kind) {
        return Ok(None);
    }
    let step_key = format!("canvas_node:{}", node.id);
    let output = checkpoints
        .and_then(|rows| {
            rows.iter().rev().find(|checkpoint| {
                checkpoint.step_key == step_key && checkpoint.status == "completed"
            })
        })
        .and_then(|checkpoint| {
            checkpoint
                .checkpoint_json
                .as_ref()
                .or(checkpoint.artifacts_json.as_ref())
        })
        .ok_or_else(|| unresolved_upstream_operation_error(consumer_label))?;
    Ok(Some(output))
}

fn unresolved_upstream_operation_error(consumer_label: &str) -> String {
    format!(
        "{consumer_label} 不能直接读取未运行的上游操作节点，请先使用“运行到此节点”或整图运行生成当前 workflow checkpoint"
    )
}
