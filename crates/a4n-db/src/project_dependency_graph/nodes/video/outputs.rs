use serde_json::json;

use super::stats::VideoPromptStats;
use crate::project_dependency_graph::graph::{GeneratedImageGraphRow, GeneratedVideoGraphRow};
use crate::project_dependency_graph::{
    builder::{GraphBuilder, NodeSeedMetadata, node},
    helpers::{count_strings, first_available_graph_source, video_frame_image_prompt_id},
};

pub(super) fn add_video_output_nodes(
    graph: &mut GraphBuilder,
    chapter_count: u64,
    generated_images: &[GeneratedImageGraphRow],
    generated_videos: &[GeneratedVideoGraphRow],
    prompt_stats: &VideoPromptStats,
) {
    let frame_image_count = generated_images
        .iter()
        .filter(|image| {
            video_frame_image_prompt_id(&image.image_type, image.metadata_json.as_ref()).is_some()
        })
        .count() as u64;
    graph.add_node(node(
        "video_frame_images",
        "image",
        "video_frame_image",
        "视频首尾帧图片",
        frame_image_count,
        11,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some("由首尾帧提示词生成的视频边界帧图片。".to_string()),
            details: json!({}),
        },
    ));
    if let Some(source) = first_available_graph_source(&[
        ("video_frame_prompts", prompt_stats.frame_prompt_count),
        ("video_segment_plans", prompt_stats.segment_plan_count),
    ]) {
        graph.add_edge(source, "video_frame_images", "generates", frame_image_count);
    }

    graph.add_node(node(
        "chapter_video_prompts",
        "prompt",
        "chapter_video_prompt",
        "视频生成提示词",
        prompt_stats.chapter_video_prompt_count,
        12,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(
                "由拍摄剧本、执行片段规划、首尾帧提示词和参考素材派生的视频生成提示词。"
                    .to_string(),
            ),
            details: json!({}),
        },
    ));
    if let Some(source) = first_available_graph_source(&[
        ("video_frame_images", frame_image_count),
        ("video_frame_prompts", prompt_stats.frame_prompt_count),
        ("video_segment_plans", prompt_stats.segment_plan_count),
        ("chapters", chapter_count),
    ]) {
        graph.add_edge(
            source,
            "chapter_video_prompts",
            "derives",
            prompt_stats.chapter_video_prompt_count,
        );
    }

    let generated_video_count = generated_videos.len() as u64;
    graph.add_node(node(
        "generated_videos",
        "video",
        "generated_video",
        "生成视频",
        generated_video_count,
        13,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some("由视频生成提示词、首尾帧和参考素材生成或导入的视频成品。".to_string()),
            details: json!({
                "linked_prompt_count": prompt_stats.linked_video_count,
                "statuses": count_strings(generated_videos.iter().map(|row| row.status.as_str())),
            }),
        },
    ));
    graph.add_edge(
        "chapter_video_prompts",
        "generated_videos",
        "generates",
        prompt_stats.linked_video_count.max(generated_video_count),
    );
}
