use std::collections::BTreeSet;

use a4n_core::models::{
    ai::generated_image,
    chapter::{chapter, chapter_video_prompt},
};
use chrono::Utc;
use serde_json::json;

use super::add_video_nodes;
use crate::project_dependency_graph::builder::{GraphBuilder, NodeSeedMetadata, node};

fn video_prompt_model(
    id: i64,
    prompt_type: &str,
    workflow_mode: &str,
) -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id,
        project_id: 2,
        chapter_id: 1,
        style: None,
        prompt_type: prompt_type.to_string(),
        workflow_mode: workflow_mode.to_string(),
        shot_number: 1,
        title: Some(prompt_type.to_string()),
        scene_summary: Some("雨夜开端".to_string()),
        visual_prompt: "{}".to_string(),
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 5,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: None,
        status: "completed".to_string(),
        generated_video_id: None,
        timeline_item_id: None,
        metadata_json: None,
        created_at: now,
        updated_at: now,
    }
}

fn generated_frame_image(id: i64, prompt_id: i64) -> generated_image::Model {
    let now = Utc::now().naive_utc();
    generated_image::Model {
        id,
        project_id: 2,
        image_type: "video_first_frame".to_string(),
        file_path: format!("video/frame-{id}.png"),
        prompt: "frame".to_string(),
        model: "test".to_string(),
        width: 1024,
        height: 576,
        file_size: Some(100),
        ai_score: None,
        is_favorite: false,
        chapter_id: Some(1),
        panel_id: None,
        metadata_json: Some(json!({"prompt_id": prompt_id})),
        created_at: now,
    }
}

#[test]
fn video_graph_edges_follow_primary_chain_without_cross_level_shortcuts() {
    let now = Utc::now().naive_utc();
    let chapters = vec![chapter::Model {
        id: 1,
        project_id: 1,
        outline_id: None,
        chapter_number: 1,
        title: Some("第一章".to_string()),
        content: "他推开门，走进雨夜。".to_string(),
        word_count: 10,
        status: chapter::STATUS_IMPORTED.to_string(),
        summary: Some("雨夜开端".to_string()),
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }];
    let prompts = vec![
        video_prompt_model(1, "frame_segment_plan", "boundary_frames"),
        video_prompt_model(2, "frame_prompt", "boundary_frames"),
        video_prompt_model(3, "video_prompt", "boundary_frames"),
    ];
    let images = vec![generated_frame_image(10, 3)];
    let mut graph = GraphBuilder::default();
    graph.add_node(node(
        "chapters",
        "text",
        "chapter_content",
        "章节正文",
        1,
        5,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: None,
            details: json!({}),
        },
    ));

    add_video_nodes(&mut graph, &chapters, &[], &prompts, &images, &[]);
    let graph = graph.finish(1).expect("graph should be acyclic");
    let edges = graph
        .edges
        .iter()
        .map(|edge| (edge.from.as_str(), edge.to.as_str(), edge.relation.as_str()))
        .collect::<BTreeSet<_>>();

    assert!(edges.contains(&("chapters", "video_segment_plans", "derives")));
    assert!(edges.contains(&("video_segment_plans", "video_frame_prompts", "derives")));
    assert!(edges.contains(&("video_frame_prompts", "video_frame_images", "generates")));
    assert!(edges.contains(&("video_frame_images", "chapter_video_prompts", "derives")));
    assert!(!edges.iter().any(|(from, to, _)| {
        matches!(
            (*from, *to),
            ("video_segment_plans", "chapter_video_prompts")
                | ("video_frame_prompts", "chapter_video_prompts")
        )
    }));
}
