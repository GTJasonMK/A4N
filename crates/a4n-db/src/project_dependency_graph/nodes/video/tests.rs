use std::collections::BTreeSet;

use serde_json::json;

use super::add_video_nodes;
use crate::project_dependency_graph::builder::{GraphBuilder, NodeSeedMetadata, node};
use crate::project_dependency_graph::graph::{
    ChapterGraphRow, GeneratedImageGraphRow, ShootingScriptGraphRow, VideoPromptGraphRow,
    VisualPlanGraphRow,
};

fn video_prompt_row(prompt_type: &str) -> VideoPromptGraphRow {
    VideoPromptGraphRow {
        prompt_type: prompt_type.to_string(),
        generated_video_id: None,
    }
}

fn generated_frame_image(prompt_id: i64) -> GeneratedImageGraphRow {
    GeneratedImageGraphRow {
        image_type: a4n_domain::image::IMAGE_TYPE_VIDEO_START_FRAME.to_string(),
        metadata_json: Some(json!({
            "generation_context": {
                "source": a4n_domain::image::IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
                "prompt_id": prompt_id
            }
        })),
    }
}

fn visual_plan_model() -> VisualPlanGraphRow {
    VisualPlanGraphRow {
        source_chapter_id: 1,
        source_snapshot_id: None,
        source_content_sha256: "content".to_string(),
        status: "confirmed".to_string(),
    }
}

fn shooting_script_row() -> ShootingScriptGraphRow {
    ShootingScriptGraphRow {
        source_chapter_id: 1,
        source_snapshot_id: None,
        source_content_sha256: "content".to_string(),
    }
}

#[test]
fn video_graph_edges_follow_primary_chain_without_cross_level_shortcuts() {
    let chapters = vec![ChapterGraphRow {
        id: 1,
        content: "他推开门，走进雨夜。".to_string(),
        word_count: 10,
        has_analysis: false,
    }];
    let prompts = vec![
        video_prompt_row("frame_segment_plan"),
        video_prompt_row("frame_prompt"),
        video_prompt_row("video_prompt"),
    ];
    let images = vec![generated_frame_image(3)];
    let visual_plans = vec![visual_plan_model()];
    let shooting_scripts = vec![shooting_script_row()];
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

    add_video_nodes(
        &mut graph,
        &chapters,
        &visual_plans,
        &shooting_scripts,
        &prompts,
        &images,
        &[],
    );
    let graph = graph.finish(1).expect("graph should be acyclic");
    let edges = graph
        .edges
        .iter()
        .map(|edge| (edge.from.as_str(), edge.to.as_str(), edge.relation.as_str()))
        .collect::<BTreeSet<_>>();

    assert!(edges.contains(&("chapters", "chapter_visual_plans", "directs")));
    assert!(edges.contains(&("chapter_visual_plans", "chapter_shooting_script", "derives")));
    assert!(edges.contains(&("chapter_shooting_script", "video_segment_plans", "derives")));
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
