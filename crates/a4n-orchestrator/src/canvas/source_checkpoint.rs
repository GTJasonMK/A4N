use a4n_db_models::models::{canvas::canvas_node, generation::workflow_checkpoint};
use a4n_domain::CanvasNodeRefKind;
use serde_json::Value;

use crate::{
    canvas::{canvas_node_has_persisted_runtime_result, canvas_node_is_operation},
    workflow::WORKFLOW_STATUS_COMPLETED,
};

pub(crate) fn source_operation_checkpoint_output<'a>(
    checkpoints: Option<&'a [workflow_checkpoint::Model]>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<&'a Value>, String> {
    if !canvas_node_is_operation(&node.node_kind) {
        return Ok(None);
    }
    let step_key = crate::canvas::canvas_node_step_key(node.id)?;
    if let Some(output) = checkpoints
        .and_then(|rows| {
            rows.iter().rev().find(|checkpoint| {
                checkpoint.step_key == step_key && checkpoint.status == WORKFLOW_STATUS_COMPLETED
            })
        })
        .and_then(|checkpoint| checkpoint.artifacts_json.as_ref())
    {
        return Ok(Some(output));
    }
    if canvas_node_has_persisted_runtime_result(node) {
        return Ok(None);
    }
    Err(unresolved_upstream_operation_error(consumer_label))
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
    if let Some(value) = node
        .ref_type
        .as_deref()
        .filter(|value| *value == ref_kind.as_str())
        .and(node.ref_id)
    {
        return Ok(Some(value));
    }
    if canvas_node_has_persisted_runtime_result(node) {
        return Ok(node.config_json.get(output_key).and_then(Value::as_i64));
    }
    Ok(None)
}

fn unresolved_upstream_operation_error(consumer_label: &str) -> String {
    format!(
        "{consumer_label} 不能直接读取未运行的上游操作节点，请先使用“运行到此节点”或整图运行生成当前 workflow checkpoint"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn node(config_json: Value) -> canvas_node::Model {
        let now = Utc::now().naive_utc();
        canvas_node::Model {
            id: 7,
            graph_id: 3,
            node_kind: "GenerateVideoNode".to_string(),
            ref_type: None,
            ref_id: None,
            config_json,
            position_json: json!({"x": 0, "y": 0}),
            state: "completed".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn persisted_runtime_result_is_available_without_current_checkpoint() {
        let source = node(json!({
            "canvas_runtime_result": true,
            "generated_video_id": 91,
        }));
        assert_eq!(
            source_node_output_id(
                None,
                &source,
                "AssembleVideoNode",
                "generated_video_id",
                CanvasNodeRefKind::GeneratedVideo,
            )
            .expect("持久化结果应可读取"),
            Some(91)
        );
    }

    #[test]
    fn missing_runtime_result_still_requires_checkpoint() {
        let source = node(json!({"generated_video_id": 91}));
        let error = source_node_output_id(
            None,
            &source,
            "AssembleVideoNode",
            "generated_video_id",
            CanvasNodeRefKind::GeneratedVideo,
        )
        .expect_err("未运行节点不能读取普通 config 字段");
        assert!(error.contains("当前 workflow checkpoint"));
    }
}
