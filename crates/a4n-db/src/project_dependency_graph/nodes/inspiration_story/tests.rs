use a4n_core::models::{
    chapter::{chapter, chapter_outline},
    project::{blueprint_version, novel_blueprint},
    user::conversation,
};
use chrono::Utc;
use serde_json::json;

use super::{add_inspiration_nodes, add_story_nodes};
use crate::project_dependency_graph::{
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
    let now = Utc::now().naive_utc();
    let conversations = [
        conversation::Model {
            id: 1,
            project_id: 7,
            title: None,
            conversation_type: "inspiration".to_string(),
            summary: None,
            extracted_facts: None,
            message_count: 3,
            is_active: true,
            created_at: now,
            updated_at: now,
        },
        conversation::Model {
            id: 2,
            project_id: 7,
            title: None,
            conversation_type: "inspiration".to_string(),
            summary: None,
            extracted_facts: None,
            message_count: -4,
            is_active: true,
            created_at: now,
            updated_at: now,
        },
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
    let now = Utc::now().naive_utc();
    let blueprints = [novel_blueprint::Model {
        id: 1,
        project_id: 7,
        title: "蓝图".to_string(),
        genre: "悬疑".to_string(),
        style: None,
        tone: None,
        target_audience: None,
        one_sentence_summary: Some("一句话梗概".to_string()),
        full_synopsis: None,
        world_setting: None,
        current_version: "1.0".to_string(),
        needs_part_outlines: false,
        total_chapters: Some(2),
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        created_at: now,
        updated_at: now,
    }];
    let versions = [blueprint_version::Model {
        id: 1,
        project_id: 7,
        version: "1.0".to_string(),
        change_description: None,
        blueprint_data: json!({}),
        world_setting_snapshot: None,
        characters_snapshot: None,
        relationships_snapshot: None,
        synopsis_snapshot: None,
        created_at: now,
        created_by: None,
    }];
    let outlines = [chapter_outline::Model {
        id: 1,
        project_id: 7,
        part_id: None,
        chapter_number: 2,
        title: Some("第二章".to_string()),
        summary: None,
        beats: None,
        key_scenes: None,
        pacing_score: None,
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: None,
        status: chapter_outline::STATUS_COMPLETED.to_string(),
        created_at: now,
        updated_at: now,
    }];
    let chapters = [
        chapter::Model {
            id: 1,
            project_id: 7,
            outline_id: Some(1),
            chapter_number: 1,
            title: None,
            content: "正文".to_string(),
            word_count: 120,
            status: chapter::STATUS_COMPLETED.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: Some(json!({ "beats": ["开端"] })),
            created_at: now,
            updated_at: now,
        },
        chapter::Model {
            id: 2,
            project_id: 7,
            outline_id: None,
            chapter_number: 2,
            title: None,
            content: String::new(),
            word_count: -10,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: Some(serde_json::Value::Null),
            created_at: now,
            updated_at: now,
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
    assert_eq!(graph_node(&graph, "chapter_analysis").count, 1);
    assert!(graph.edges.iter().any(|edge| {
        edge.from == "blueprint"
            && edge.to == "chapter_outlines"
            && edge.relation == "derives"
            && edge.count == 1
    }));
    assert!(graph.edges.iter().all(|edge| edge.from != "part_outlines"));
}
