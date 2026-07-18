use a4n_core::models::chapter::{chapter, chapter_shooting_script};
use serde_json::json;

use super::stats::VideoPromptStats;
use crate::project_dependency_graph::{
    builder::{GraphBuilder, NodeSeedMetadata, node},
    helpers::first_available_graph_source,
};

pub(super) fn add_video_planning_nodes(
    graph: &mut GraphBuilder,
    chapters: &[chapter::Model],
    shooting_scripts: &[chapter_shooting_script::Model],
    prompt_stats: &VideoPromptStats,
) {
    let shooting_script_count = shooting_scripts.len() as u64;
    graph.add_node(node(
        "chapter_shooting_script",
        "text",
        "chapter_shooting_script",
        "章节拍摄剧本",
        shooting_script_count,
        8,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some("由章节正文改编并发布的结构化可拍摄剧本。".to_string()),
            details: json!({}),
        },
    ));
    if !chapters.is_empty() {
        graph.add_edge(
            "chapters",
            "chapter_shooting_script",
            "derives",
            shooting_script_count,
        );
    }

    graph.add_node(node(
        "video_segment_plans",
        "text",
        "video_segment_plan",
        "视频分段规划",
        prompt_stats.segment_plan_count,
        9,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(
                "由章节拍摄剧本派生的执行片段规划，决定每个视频片段的意图、节奏和首尾帧数量。"
                    .to_string(),
            ),
            details: json!({}),
        },
    ));
    if let Some(source) = first_available_graph_source(&[
        ("chapter_shooting_script", shooting_script_count),
        ("chapters", chapters.len() as u64),
    ]) {
        graph.add_edge(
            source,
            "video_segment_plans",
            "derives",
            prompt_stats.segment_plan_count,
        );
    }

    graph.add_node(node(
        "video_frame_prompts",
        "prompt",
        "video_frame_prompt",
        "视频首尾帧提示词",
        prompt_stats.frame_prompt_count,
        10,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some("由片段规划派生的边界关键帧/首帧图片提示词。".to_string()),
            details: json!({}),
        },
    ));
    if let Some(source) = first_available_graph_source(&[
        ("video_segment_plans", prompt_stats.segment_plan_count),
        ("chapters", chapters.len() as u64),
    ]) {
        graph.add_edge(
            source,
            "video_frame_prompts",
            "derives",
            prompt_stats.frame_prompt_count,
        );
    }
}
