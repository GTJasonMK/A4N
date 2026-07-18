use super::super::super::rules::{
    canvas_config_i64, canvas_config_string, i64_to_i32, incoming_edges,
};
use super::super::super::sources::{source_generated_image_id, source_text_value};
use super::modes::{generate_video_input_mode, validate_generate_video_image_inputs};
use super::ports::{set_single_image_input, video_image_input_port};
use crate::actions::video_generation::GenerateCanvasVideoInput;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) async fn build_canvas_video_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<GenerateCanvasVideoInput, String> {
    let incoming = incoming_edges(document, node.id);
    let mut prompt_parts = Vec::new();
    let mut first_frame_image_id = None;
    let mut last_frame_image_id = None;
    let mut reference_image_ids = Vec::new();
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
            "GenerateVideoNode",
        )
        .await?
        {
            prompt_parts.push(prompt);
        }
        if let Some(image_id) =
            source_generated_image_id(state, workflow_run_id, source, "GenerateVideoNode").await?
        {
            match video_image_input_port(edge)? {
                "first_frame" => set_single_image_input(
                    &mut first_frame_image_id,
                    image_id,
                    "GenerateVideoNode 首帧输入",
                )?,
                "end_frame" => set_single_image_input(
                    &mut last_frame_image_id,
                    image_id,
                    "GenerateVideoNode 尾帧输入",
                )?,
                "reference_image" if !reference_image_ids.contains(&image_id) => {
                    reference_image_ids.push(image_id);
                }
                _ => {}
            }
        }
    }
    let config_id = canvas_config_i64(&node.config_json, "video_config_id");

    let prompt = prompt_parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if prompt.trim().is_empty() {
        return Err("GenerateVideoNode 缺少 prompt 输入".to_string());
    }
    let input_mode = generate_video_input_mode(node.config_json.get("input_mode"))?;
    validate_generate_video_image_inputs(
        input_mode,
        first_frame_image_id,
        last_frame_image_id,
        &reference_image_ids,
    )?;
    Ok(GenerateCanvasVideoInput {
        user_id,
        project_id,
        graph_id,
        node_id: node.id,
        workflow_run_id: None,
        prompt,
        negative_prompt: canvas_config_string(&node.config_json, &["negative_prompt"]),
        config_id,
        width: canvas_config_i64(&node.config_json, "width").and_then(i64_to_i32),
        height: canvas_config_i64(&node.config_json, "height").and_then(i64_to_i32),
        duration_secs: canvas_config_i64(&node.config_json, "duration_secs").and_then(i64_to_i32),
        fps: canvas_config_i64(&node.config_json, "fps").and_then(i64_to_i32),
        first_frame_image_id,
        last_frame_image_id,
        reference_image_ids,
        source_node_ids,
    })
}
