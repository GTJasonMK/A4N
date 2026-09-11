use a4n_db_models::models::{canvas::canvas_node, generation::workflow_checkpoint};
use a4n_domain::CanvasNodeRefKind;
use serde_json::Value;

use crate::{canvas::canvas_node_is_operation, workflow::WORKFLOW_STATUS_COMPLETED};

pub(crate) fn source_operation_checkpoint_output<'a>(
    checkpoints: Option<&'a [workflow_checkpoint::Model]>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<&'a Value>, String> {
    if !canvas_node_is_operation(&node.node_kind) {
        return Ok(None);
    }
    let step_key = crate::canvas::canvas_node_step_key(node.id)?;
    let output = checkpoints
        .and_then(|rows| {
            rows.iter().rev().find(|checkpoint| {
                checkpoint.step_key == step_key && checkpoint.status == WORKFLOW_STATUS_COMPLETED
            })
        })
        .and_then(|checkpoint| checkpoint.artifacts_json.as_ref())
        .ok_or_else(|| unresolved_upstream_operation_error(consumer_label))?;
    Ok(Some(output))
}

pub(crate) fn source_node_output_id(
    checkpoints: Option<&[workflow_checkpoint::Model]>,
    node: &canvas_node::Model,
    consumer_label: &str,
    output_key: &str,
    ref_kind: CanvasNodeRefKind,
) -> Result<Option<i64>, String> {
    if let Some(output) = source_operation_checkpoint_output(checkpoints, node, consumer_label)? {
        return Ok(output.get(output_key).and_then(Value::as_i64));
    }
    Ok(node
        .ref_type
        .as_deref()
        .filter(|value| *value == ref_kind.as_str())
        .and(node.ref_id))
}

fn unresolved_upstream_operation_error(consumer_label: &str) -> String {
    format!(
        "{consumer_label} 不能直接读取未运行的上游操作节点，请先使用“运行到此节点”或整图运行生成当前 workflow checkpoint"
    )
}
