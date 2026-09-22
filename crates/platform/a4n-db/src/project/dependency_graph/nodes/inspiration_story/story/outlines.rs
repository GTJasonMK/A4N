use serde_json::json;

use crate::project::dependency_graph::graph::{ChapterOutlineGraphRow, PartOutlineGraphRow};
use crate::project::dependency_graph::helpers::outline_range_summary;
use crate::project::dependency_graph::{GraphBuilder, NodeSeedMetadata, node};

pub(super) fn add_outline_nodes(
    graph: &mut GraphBuilder,
    part_outlines: &[PartOutlineGraphRow],
    chapter_outlines: &[ChapterOutlineGraphRow],
) {
    graph.add_node(node(
        "part_outlines",
        "text",
        "part_outline",
        "分部大纲",
        part_outlines.len() as u64,
        3,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: outline_range_summary(
                part_outlines.first().map(|row| row.start_chapter),
                part_outlines.last().map(|row| row.end_chapter),
            ),
            details: json!({}),
        },
    ));
    graph.add_edge(
        "blueprint",
        "part_outlines",
        "derives",
        part_outlines.len() as u64,
    );

    graph.add_node(node(
        "chapter_outlines",
        "text",
        "chapter_outline",
        "章节大纲",
        chapter_outlines.len() as u64,
        4,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: outline_range_summary(
                chapter_outlines.first().map(|row| row.chapter_number),
                chapter_outlines.last().map(|row| row.chapter_number),
            ),
            details: json!({}),
        },
    ));
    if part_outlines.is_empty() {
        graph.add_edge(
            "blueprint",
            "chapter_outlines",
            "derives",
            chapter_outlines.len() as u64,
        );
        return;
    }
    graph.add_edge(
        "part_outlines",
        "chapter_outlines",
        "derives",
        chapter_outlines.len() as u64,
    );
}
