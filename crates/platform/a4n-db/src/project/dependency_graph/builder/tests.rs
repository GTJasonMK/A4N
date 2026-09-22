use std::collections::BTreeSet;

use serde_json::json;

use super::{
    GraphBuilder,
    seed::{NodeSeedMetadata, node},
    topology::find_cycle_hint,
};
use crate::project::dependency_graph::AssetGraphEdge;

fn seed(id: &str, count: u64, layer: i32, editable: bool) -> super::seed::NodeSeed {
    node(
        id,
        "test",
        "test_asset",
        id,
        count,
        layer,
        NodeSeedMetadata {
            status: None,
            editable,
            needs_visualization: false,
            visualized: false,
            summary: None,
            details: json!({}),
        },
    )
}

#[test]
fn graph_builder_projects_edges_downstream_and_mutation_policy() {
    let mut graph = GraphBuilder::default();
    graph.add_node(seed("project", 1, 0, false));
    graph.add_node(seed("middle", 1, 1, true));
    graph.add_node(seed("leaf", 1, 2, true));
    graph.add_node(seed("empty", 0, 3, true));

    graph.add_edge("project", "middle", "derives", 2);
    graph.add_edge("project", "middle", "derives", 3);
    graph.add_edge("middle", "leaf", "generates", 1);
    graph.add_edge("leaf", "leaf", "invalid", 1);
    graph.add_edge("missing", "leaf", "invalid", 1);
    graph.add_edge("leaf", "middle", "invalid", 1);

    let graph = graph.finish(42).unwrap();
    assert_eq!(graph.project_id, 42);
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
    assert_eq!(graph.layers.len(), 3);
    assert_eq!(graph.layers[0].label, "项目");
    assert_eq!(graph.layers[1].label, "灵感");
    assert_eq!(graph.layers[2].label, "蓝图");

    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.from == "project" && edge.to == "middle")
        .unwrap();
    assert_eq!(edge.count, 5);

    let node = |id: &str| graph.nodes.iter().find(|node| node.id == id).unwrap();
    assert_eq!(node("project").direct_downstream_count, 1);
    assert_eq!(node("project").downstream_count, 2);
    assert!(!node("project").mutable);
    assert_eq!(node("project").mutation_policy, "readonly_aggregate");
    assert_eq!(node("middle").direct_downstream_count, 1);
    assert_eq!(node("middle").downstream_count, 1);
    assert!(!node("middle").mutable);
    assert_eq!(
        node("middle").mutation_policy,
        "requires_conflict_resolution"
    );
    assert_eq!(node("leaf").direct_downstream_count, 0);
    assert_eq!(node("leaf").downstream_count, 0);
    assert!(node("leaf").mutable);
    assert_eq!(node("leaf").mutation_policy, "leaf_editable");
    assert_eq!(graph.warnings.len(), 3);
    assert!(
        graph
            .warnings
            .iter()
            .any(|warning| warning.contains("自循环"))
    );
    assert!(
        graph
            .warnings
            .iter()
            .any(|warning| warning.contains("缺少上游节点"))
    );
    assert!(
        graph
            .warnings
            .iter()
            .any(|warning| warning.contains("不符合拓扑层级"))
    );
}

#[test]
fn cycle_hint_reports_only_nodes_left_with_positive_indegree() {
    let nodes = BTreeSet::from(["a".to_string(), "b".to_string(), "root".to_string()]);
    let edges = vec![
        AssetGraphEdge {
            from: "root".to_string(),
            to: "a".to_string(),
            relation: "derives".to_string(),
            count: 1,
        },
        AssetGraphEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            relation: "derives".to_string(),
            count: 1,
        },
        AssetGraphEdge {
            from: "b".to_string(),
            to: "a".to_string(),
            relation: "derives".to_string(),
            count: 1,
        },
    ];

    assert_eq!(
        find_cycle_hint(nodes, &edges),
        Some(vec!["a".to_string(), "b".to_string()])
    );
}
