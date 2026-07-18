use a4n_core::models::project::{blueprint_version, novel_blueprint};
use serde_json::json;

use crate::project_dependency_graph::{GraphBuilder, NodeSeedMetadata, node};

pub(super) fn add_blueprint_nodes(
    graph: &mut GraphBuilder,
    blueprints: &[novel_blueprint::Model],
    blueprint_versions: &[blueprint_version::Model],
) {
    graph.add_node(node(
        "blueprint",
        "text",
        "novel_blueprint",
        "故事蓝图",
        blueprints.len() as u64,
        2,
        NodeSeedMetadata {
            status: blueprints.first().map(|row| row.current_version.clone()),
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: blueprints
                .first()
                .and_then(|row| row.one_sentence_summary.clone()),
            details: json!({ "version_count": blueprint_versions.len() }),
        },
    ));
    graph.add_edge(
        "inspiration",
        "blueprint",
        "derives",
        blueprints.len() as u64,
    );

    if blueprint_versions.is_empty() {
        return;
    }

    graph.add_node(node(
        "blueprint_versions",
        "text",
        "blueprint_version",
        "蓝图版本",
        blueprint_versions.len() as u64,
        3,
        NodeSeedMetadata {
            status: blueprint_versions.last().map(|row| row.version.clone()),
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some("蓝图快照历史，用于对比和恢复。".to_string()),
            details: json!({}),
        },
    ));
    graph.add_edge(
        "blueprint",
        "blueprint_versions",
        "snapshots",
        blueprint_versions.len() as u64,
    );
}
