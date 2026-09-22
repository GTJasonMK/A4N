use serde_json::json;

use super::{add_inspiration_nodes, add_story_nodes};
use crate::project::dependency_graph::graph::{
    BlueprintGraphRow, BlueprintVersionGraphRow, ChapterGraphRow, ChapterOutlineGraphRow,
    ConversationGraphRow,
};
use crate::project::dependency_graph::{
    AssetDependencyGraph, AssetGraphNode, GraphBuilder, NodeSeedMetadata, node,
};

fn add_seed(graph: &mut GraphBuilder, id: &str, layer: i32) {
    graph.add_node(node(
        id,
        "text",
        "seed",
        id,
        1,
        layer,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: false,
            visualized: false,
            summary: None,
            details: json!({}),
        },
    ));
}

fn graph_node<'a>(graph: &'a AssetDependencyGraph, id: &str) -> &'a AssetGraphNode {
    graph
        .nodes
        .iter()
        .find(|candidate| candidate.id == id)
        .unwrap_or_else(|| panic!("missing graph node {id}"))
}

#[test]
fn inspiration_nodes_aggregate_non_negative_message_counts() {
    let conversations = [
        ConversationGraphRow { message_count: 3 },
        ConversationGraphRow { message_count: -4 },
    ];
    let mut builder = GraphBuilder::default();
    add_seed(&mut builder, "project", 0);

    add_inspiration_nodes(&mut builder, &conversations);
    let graph = builder.finish(7).unwrap();

    let inspiration = graph_node(&graph, "inspiration");
    assert_eq!(inspiration.count, 2);
    assert_eq!(inspiration.details, json!({ "message_count": 3 }));
    assert_eq!(
        inspiration.summary.as_deref(),
        Some("2 个对话，3 条消息，是蓝图生成的上游来源。")
    );
    assert!(graph.edges.iter().any(|edge| {
        edge.from == "project"
            && edge.to == "inspiration"
            && edge.relation == "contains"
            && edge.count == 2
    }));
}

#[test]
fn story_nodes_preserve_outline_fallback_word_count_and_analysis_count() {
    let blueprints = [BlueprintGraphRow {
        one_sentence_summary: Some("一句话梗概".to_string()),
        current_version: "1.0".to_string(),
    }];
    let versions = [BlueprintVersionGraphRow {
        version: "1.0".to_string(),
    }];
    let outlines = [ChapterOutlineGraphRow { chapter_number: 2 }];
    let chapters = [
        ChapterGraphRow {
            id: 1,
            content: "正文".to_string(),
            word_count: 120,
            has_analysis: true,
        },
        ChapterGraphRow {
            id: 2,
            content: String::new(),
            word_count: -10,
            has_analysis: false,
        },
    ];
    let mut builder = GraphBuilder::default();
    add_seed(&mut builder, "inspiration", 1);

    add_story_nodes(
        &mut builder,
        &blueprints,
        &versions,
        &[],
        &outlines,
        &chapters,
    );
    let graph = builder.finish(7).unwrap();

    assert_eq!(
        graph_node(&graph, "blueprint").status.as_deref(),
        Some("1.0")
    );
    assert_eq!(graph_node(&graph, "blueprint_versions").count, 1);
    assert!(graph.nodes.iter().all(|node| node.id != "part_outlines"));
    assert_eq!(
        graph_node(&graph, "chapter_outlines").summary.as_deref(),
        Some("覆盖第 2 到第 2 章。")
    );
    assert_eq!(
        graph_node(&graph, "chapters").details,
        json!({ "word_count": 120 })
    );
    let chapter_analysis = graph_node(&graph, "chapter_analysis");
    assert_eq!(chapter_analysis.count, 1);
    assert!(!chapter_analysis.mutable);
    assert_eq!(chapter_analysis.mutation_policy, "readonly_aggregate");
    assert!(graph.edges.iter().any(|edge| {
        edge.from == "blueprint"
            && edge.to == "chapter_outlines"
            && edge.relation == "derives"
            && edge.count == 1
    }));
    assert!(graph.edges.iter().all(|edge| edge.from != "part_outlines"));
}
