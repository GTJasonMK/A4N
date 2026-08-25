use super::structural_source_refs_for_node;
use super::video::{
    VideoPromptSourceRecord, video_prompt_asset_kind, video_prompt_sources_from_record,
};
use a4n_domain::project::{AssetDependencyGraph, AssetGraphEdge, AssetGraphLayer, AssetGraphNode};
use serde_json::json;

fn graph_node(id: &str, label: &str) -> AssetGraphNode {
    AssetGraphNode {
        id: id.to_string(),
        kind: "text".to_string(),
        asset_type: "chapter_content".to_string(),
        label: label.to_string(),
        count: 1,
        status: None,
        layer: 0,
        chapter_id: None,
        profile_id: None,
        record_id: Some(1),
        mutable: true,
        direct_downstream_count: 0,
        downstream_count: 0,
        mutation_policy: "none".to_string(),
        needs_visualization: false,
        visualized: false,
        summary: None,
        details: json!({}),
    }
}

#[test]
fn structural_source_refs_index_nodes_once_and_preserve_edge_order() {
    let graph = AssetDependencyGraph {
        project_id: 1,
        nodes: vec![
            graph_node("source-1", "首条来源"),
            graph_node("source-1", "重复来源"),
            graph_node("source-2", "第二来源"),
        ],
        edges: vec![
            AssetGraphEdge {
                from: "source-2".to_string(),
                to: "target".to_string(),
                relation: "input".to_string(),
                count: 1,
            },
            AssetGraphEdge {
                from: "source-1".to_string(),
                to: "target".to_string(),
                relation: "input".to_string(),
                count: 1,
            },
            AssetGraphEdge {
                from: "missing".to_string(),
                to: "target".to_string(),
                relation: "input".to_string(),
                count: 1,
            },
        ],
        layers: vec![AssetGraphLayer {
            index: 0,
            label: "测试".to_string(),
            node_ids: vec!["target".to_string()],
        }],
        warnings: Vec::new(),
    };

    let refs = structural_source_refs_for_node(&graph, "target");
    assert_eq!(
        refs.iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["第二来源", "首条来源"]
    );
}

#[test]
fn video_prompt_asset_kind_preserves_graph_layer_contract() {
    assert_eq!(
        video_prompt_asset_kind("frame_segment_plan"),
        "video_segment_plan"
    );
    assert_eq!(
        video_prompt_asset_kind("first_frame_prompt"),
        "video_frame_prompt"
    );
    assert_eq!(
        video_prompt_asset_kind("video_prompt"),
        "chapter_video_prompt"
    );
}

#[test]
fn video_prompt_source_projection_preserves_source_texts() {
    let prompt = VideoPromptSourceRecord {
        id: 7,
        visual_prompt: "视觉".to_string(),
        motion_prompt: Some("运动".to_string()),
        negative_prompt: Some("排除".to_string()),
    };
    let sources = video_prompt_sources_from_record(&prompt);
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0].source_record_id, Some(7));
    assert_eq!(sources[0].text, "视觉");
    assert_eq!(sources[1].text, "运动");
    assert_eq!(sources[2].text, "排除");
}
