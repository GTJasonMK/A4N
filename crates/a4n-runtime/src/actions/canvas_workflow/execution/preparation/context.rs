use super::super::graph::canvas_node_kind_is_output;
use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_orchestrator::canvas::{
    CanvasGraphRecordDocument as CanvasGraphDocument,
    validate_canvas_record_document_edge_contract as validate_canvas_document_edge_contract,
};
use chrono::Utc;

pub(super) async fn load_canvas_execution_document(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
) -> Result<CanvasGraphDocument, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let document = state
        .orchestrator
        .get_or_create_canvas_graph_record(project_id, Some(graph_id), Utc::now().naive_utc())
        .await
        .map_err(app_error_message)?;
    validate_canvas_document_edge_contract(&document).map_err(|err| err.to_string())?;
    Ok(document)
}

pub(super) fn validate_canvas_output_target(
    document: &CanvasGraphDocument,
    target_node_id: Option<i64>,
) -> Result<(), String> {
    let Some(target_node_id) = target_node_id else {
        return Ok(());
    };
    let target = document
        .nodes
        .iter()
        .find(|node| node.id == target_node_id)
        .ok_or_else(|| format!("画布节点不存在: {target_node_id}"))?;
    if !canvas_node_kind_is_output(&target.node_kind) {
        return Err("运行到此节点只能锚定输出节点".to_string());
    }
    Ok(())
}
