use super::*;
use crate::actions::image_generation::GenerateCanvasImageInput;
use crate::actions::video_generation::GenerateCanvasVideoInput;
use a4n_core::models::canvas::{canvas_edge, canvas_node};
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;
use serde_json::Value;

fn block_on_input<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
        .block_on(future)
}

#[cfg(test)]
fn build_canvas_video_input(
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<GenerateCanvasVideoInput, String> {
    block_on_input(build_canvas_video_input_for_run(
        None, None, user_id, project_id, graph_id, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_image_input(
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<GenerateCanvasImageInput, String> {
    block_on_input(build_canvas_image_input_for_run(
        None, None, project_id, graph_id, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_trim_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(i64, f32, f32, Vec<i64>), String> {
    block_on_input(build_canvas_trim_input_for_run(None, None, node, document))
}

#[cfg(test)]
fn build_canvas_extract_frame_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(i64, String, Option<f32>, Vec<i64>), String> {
    block_on_input(build_canvas_extract_frame_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_assemble_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(Vec<i64>, Option<String>, Vec<i64>), String> {
    block_on_input(build_canvas_assemble_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_export_video_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasExportVideoInput, String> {
    block_on_input(build_canvas_export_video_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_prompt_artifact_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasRenderedPrompt, String> {
    block_on_input(build_canvas_prompt_artifact_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_llm_transform_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasLlmTransformInput, String> {
    block_on_input(build_canvas_llm_transform_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_output_value_for_test(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    expected_type: &str,
    consumer_label: &str,
) -> Result<CanvasOutputValue, String> {
    block_on_input(build_canvas_output_value(
        None,
        None,
        node,
        document,
        expected_type,
        consumer_label,
    ))
}

use a4n_core::models::canvas::canvas_graph;
use serde_json::json;

fn test_graph_document(
    nodes: Vec<canvas_node::Model>,
    edges: Vec<canvas_edge::Model>,
) -> CanvasGraphDocument {
    let now = chrono::Utc::now().naive_utc();
    CanvasGraphDocument {
        graph: canvas_graph::Model {
            id: 10,
            project_id: 20,
            name: "测试画布".to_string(),
            graph_kind: "video_studio".to_string(),
            version: 1,
            viewport_json: None,
            created_at: now,
            updated_at: now,
        },
        nodes,
        edges,
        text_assets: Vec::new(),
    }
}

fn test_node(id: i64, kind: &str, config_json: Value) -> canvas_node::Model {
    let now = chrono::Utc::now().naive_utc();
    canvas_node::Model {
        id,
        graph_id: 10,
        node_kind: kind.to_string(),
        ref_type: None,
        ref_id: None,
        config_json,
        position_json: json!({"x": 0, "y": 0}),
        state: "ready".to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn test_edge(source_node_id: i64, target_node_id: i64, role: &str) -> canvas_edge::Model {
    canvas_edge::Model {
        id: source_node_id * 100 + target_node_id,
        graph_id: 10,
        source_node_id,
        source_port: "output".to_string(),
        target_node_id,
        target_port: "input".to_string(),
        role: role.to_string(),
        config_json: json!({}),
    }
}

fn test_typed_edge(
    source_node_id: i64,
    target_node_id: i64,
    source_port: &str,
    target_port: &str,
    role: &str,
) -> canvas_edge::Model {
    canvas_edge::Model {
        source_port: source_port.to_string(),
        target_port: target_port.to_string(),
        ..test_edge(source_node_id, target_node_id, role)
    }
}

fn test_edge_with_order(
    source_node_id: i64,
    target_node_id: i64,
    role: &str,
    order_index: i64,
) -> canvas_edge::Model {
    canvas_edge::Model {
        config_json: json!({
            "order_index": order_index
        }),
        ..test_edge(source_node_id, target_node_id, role)
    }
}
