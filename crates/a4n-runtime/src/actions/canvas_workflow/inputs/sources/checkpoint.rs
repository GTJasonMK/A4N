use super::super::rules::canvas_node_is_operation;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use serde_json::Value;

pub(super) async fn source_operation_checkpoint_output(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<Value>, String> {
    if !canvas_node_is_operation(&node.node_kind) {
        return Ok(None);
    }
    let Some(value) = checkpoint_output_value(state, workflow_run_id, node.id).await? else {
        return Err(unresolved_upstream_operation_error(consumer_label));
    };
    Ok(Some(value))
}

async fn checkpoint_output_value(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node_id: i64,
) -> Result<Option<Value>, String> {
    let Some(state) = state else {
        return Ok(None);
    };
    let step_key = format!("canvas_node:{node_id}");
    if let Some(workflow_run_id) = workflow_run_id {
        let checkpoints = state
            .orchestrator
            .workflow_checkpoint_models(workflow_run_id)
            .await
            .map_err(|err| err.to_string())?;
        if let Some(value) = checkpoints
            .into_iter()
            .rev()
            .find(|checkpoint| checkpoint.step_key == step_key && checkpoint.status == "completed")
            .and_then(|checkpoint| checkpoint.checkpoint_json.or(checkpoint.artifacts_json))
        {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn unresolved_upstream_operation_error(consumer_label: &str) -> String {
    format!(
        "{consumer_label} 不能直接读取未运行的上游操作节点，请先使用“运行到此节点”或整图运行生成当前 workflow checkpoint"
    )
}
