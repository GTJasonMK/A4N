use a4n_db_models::models::canvas::{canvas_edge, canvas_graph, canvas_node};
use chrono::Utc;
use serde_json::json;

use super::validate_canvas_record_document_edge_contract;
use crate::canvas::graph_record::types::CanvasGraphRecordDocument;

fn graph_document(
    nodes: Vec<canvas_node::Model>,
    edges: Vec<canvas_edge::Model>,
) -> CanvasGraphRecordDocument {
    let now = Utc::now().naive_utc();
    CanvasGraphRecordDocument {
        graph: canvas_graph::Model {
            id: 1,
            project_id: 1,
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

fn node(id: i64, node_kind: &str) -> canvas_node::Model {
    let now = Utc::now().naive_utc();
    canvas_node::Model {
        id,
        graph_id: 1,
        node_kind: node_kind.to_string(),
        ref_type: None,
        ref_id: None,
        config_json: json!({}),
        position_json: json!({}),
        state: "ready".to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn assembly_edge(id: i64, source_node_id: i64, order_index: i64) -> canvas_edge::Model {
    canvas_edge::Model {
        id,
        graph_id: 1,
        source_node_id,
        source_port: "video".to_string(),
        target_node_id: 3,
        target_port: "clip".to_string(),
        role: "assembly_clip".to_string(),
        config_json: json!({ "order_index": order_index }),
    }
}

#[test]
fn document_edge_contract_rejects_missing_source_node() {
    let document = graph_document(
        vec![node(3, "AssembleVideoNode")],
        vec![assembly_edge(1, 99, 1)],
    );

    let error = validate_canvas_record_document_edge_contract(&document)
        .expect_err("missing persisted source node must be rejected");

    assert!(error.to_string().contains("引用不存在的源节点: 99"));
}

#[test]
fn document_edge_contract_accepts_contiguous_assembly_clips() {
    let document = graph_document(
        vec![
            node(1, "VideoInputNode"),
            node(2, "GenerateVideoNode"),
            node(3, "AssembleVideoNode"),
        ],
        vec![assembly_edge(1, 1, 1), assembly_edge(2, 2, 2)],
    );

    validate_canvas_record_document_edge_contract(&document)
        .expect("persisted assembly clip order should accept contiguous indexes");
}

#[test]
fn document_edge_contract_accepts_image_output_as_downstream_image_source() {
    let edge = canvas_edge::Model {
        id: 1,
        graph_id: 1,
        source_node_id: 1,
        source_port: "image".to_string(),
        target_node_id: 2,
        target_port: "first_frame".to_string(),
        role: "first_frame".to_string(),
        config_json: json!({}),
    };
    let document = graph_document(
        vec![node(1, "ImageOutputNode"), node(2, "GenerateVideoNode")],
        vec![edge],
    );

    validate_canvas_record_document_edge_contract(&document)
        .expect("image output should pass an image to a downstream image input");
}
