use super::super::rules::{canvas_config_i64, incoming_edges};
use super::super::sources::source_text_value;
use crate::actions::image_generation::GenerateCanvasImageInput;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) async fn build_canvas_image_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<GenerateCanvasImageInput, String> {
    let incoming = incoming_edges(document, node.id);
    let mut prompt_parts = Vec::new();
    let mut source_node_ids = Vec::new();

    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(prompt) = source_text_value(
            state,
            workflow_run_id,
            source,
            document,
            "GenerateImageNode",
        )
        .await?
        {
            prompt_parts.push(prompt);
        }
    }
    let config_id = canvas_config_i64(&node.config_json, "image_config_id");

    let prompt = prompt_parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if prompt.trim().is_empty() {
        return Err("GenerateImageNode 缺少 prompt 输入".to_string());
    }
    Ok(GenerateCanvasImageInput {
        project_id,
        graph_id,
        node_id: node.id,
        workflow_run_id: None,
        prompt,
        config_id,
        source_node_ids,
    })
}
