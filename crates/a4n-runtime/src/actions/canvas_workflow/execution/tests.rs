use super::prelude::*;
use super::*;

use a4n_core::models::canvas::{canvas_edge, canvas_graph};

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

fn test_node_with_id(id: i64, kind: &str) -> canvas_node::Model {
    let now = chrono::Utc::now().naive_utc();
    canvas_node::Model {
        id,
        graph_id: 10,
        node_kind: kind.to_string(),
        ref_type: None,
        ref_id: None,
        config_json: json!({}),
        position_json: json!({"x": 0, "y": 0}),
        state: "ready".to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn test_edge(source_node_id: i64, target_node_id: i64) -> canvas_edge::Model {
    canvas_edge::Model {
        id: source_node_id * 100 + target_node_id,
        graph_id: 10,
        source_node_id,
        source_port: "output".to_string(),
        target_node_id,
        target_port: "input".to_string(),
        role: "context".to_string(),
        config_json: json!({}),
    }
}

#[test]
fn canvas_node_execution_kind_rejects_non_operation_nodes() {
    assert!(canvas_node_kind_is_executable("RenderTextNode"));
    assert!(canvas_node_kind_is_executable("LlmTextNode"));
    assert!(canvas_node_kind_is_executable("GenerateVideoNode"));
    assert!(canvas_node_kind_is_executable("AssembleVideoNode"));
    assert!(canvas_node_kind_is_executable("TextProbeNode"));
    assert!(canvas_node_kind_is_executable("ImageProbeNode"));
    assert!(canvas_node_kind_is_executable("VideoProbeNode"));
    assert!(canvas_node_kind_is_executable("AudioProbeNode"));
    assert!(canvas_node_kind_is_executable("VideoOutputNode"));
    assert!(canvas_node_kind_is_output("VideoOutputNode"));

    assert!(!canvas_node_kind_is_executable("TextInputNode"));
    assert!(!canvas_node_kind_is_output("GenerateVideoNode"));
    assert!(!canvas_node_kind_is_output("TextProbeNode"));
    assert!(!canvas_node_kind_is_output("TextInputNode"));
    assert!(!canvas_node_kind_is_executable("RenderPromptNode"));
    assert!(!canvas_node_kind_is_executable("LlmTransformNode"));
    assert!(!canvas_node_kind_is_executable("AssembleNode"));
    assert!(!canvas_node_kind_is_executable("ExportVideoNode"));
    assert!(!canvas_node_kind_is_executable("TextNode"));
    assert!(!canvas_node_kind_is_executable("ConfigNode"));
    assert!(!canvas_node_kind_is_executable("MediaAssetNode"));

    assert_eq!(
        unsupported_canvas_node_execution_error("TextNode"),
        "画布节点类型不支持执行: TextNode"
    );
}

#[test]
fn canvas_graph_execution_scope_supports_full_and_to_node() {
    let document = test_graph_document(
        vec![
            test_node_with_id(1, "TextInputNode"),
            test_node_with_id(2, "LlmTextNode"),
            test_node_with_id(3, "GenerateImageNode"),
            test_node_with_id(4, "GenerateVideoNode"),
            test_node_with_id(5, "VideoOutputNode"),
        ],
        vec![
            test_edge(1, 2),
            test_edge(2, 3),
            test_edge(3, 4),
            test_edge(4, 5),
        ],
    );

    assert_eq!(
        runnable_canvas_node_ids(&document, None).unwrap(),
        vec![2, 3, 4, 5]
    );
    assert_eq!(
        runnable_canvas_node_ids(&document, Some(5)).unwrap(),
        vec![2, 3, 4, 5]
    );
    assert!(
        runnable_canvas_node_ids(&document, Some(999))
            .expect_err("missing target node should fail")
            .contains("画布节点不存在")
    );
}
