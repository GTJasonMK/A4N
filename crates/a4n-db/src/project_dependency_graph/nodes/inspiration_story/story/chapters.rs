use a4n_core::models::chapter::chapter;
use serde_json::json;

use crate::project_dependency_graph::{GraphBuilder, NodeSeedMetadata, node};

pub(super) fn add_chapter_nodes(graph: &mut GraphBuilder, chapters: &[chapter::Model]) {
    let word_count = chapters
        .iter()
        .map(|row| row.word_count.max(0) as u64)
        .sum::<u64>();
    graph.add_node(node(
        "chapters",
        "text",
        "chapter_content",
        "章节正文",
        chapters.len() as u64,
        5,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(format!(
                "{} 章正文，总字数约 {}。",
                chapters.len(),
                word_count
            )),
            details: json!({ "word_count": word_count }),
        },
    ));
    graph.add_edge(
        "chapter_outlines",
        "chapters",
        "derives",
        chapters.len() as u64,
    );

    let analysis_count = chapters
        .iter()
        .filter(|row| {
            row.analysis_data
                .as_ref()
                .is_some_and(|value| !value.is_null())
        })
        .count();
    graph.add_node(node(
        "chapter_analysis",
        "text",
        "chapter_analysis",
        "章节分析",
        analysis_count as u64,
        6,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(
                "章节正文的结构化分析，是角色、场景、物体、漫画和视频链路的重要上游。".to_string(),
            ),
            details: json!({}),
        },
    ));
    graph.add_edge(
        "chapters",
        "chapter_analysis",
        "analyzes",
        analysis_count as u64,
    );
}
