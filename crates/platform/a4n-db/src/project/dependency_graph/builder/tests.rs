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
fn graph_builder_counts_unique_descendants_across_diamond_edges() {
    let mut graph = GraphBuilder::default();
    for (id, layer) in [("root", 0), ("left", 1), ("right", 1), ("leaf", 2)] {
        graph.add_node(seed(id, 1, layer, true));
    }
    graph.add_edge("root", "left", "derives", 1);
    graph.add_edge("root", "right", "derives", 1);
    graph.add_edge("left", "leaf", "derives", 1);
    graph.add_edge("right", "leaf", "derives", 1);

    let graph = graph.finish(42).unwrap();
    let downstream_count = |id| {
        graph
            .nodes
            .iter()
            .find(|node| node.id == id)
            .unwrap()
            .downstream_count
    };
    assert_eq!(downstream_count("root"), 3);
    assert_eq!(downstream_count("left"), 1);
    assert_eq!(downstream_count("right"), 1);
    assert_eq!(downstream_count("leaf"), 0);
}

#[test]
#[ignore = "manual performance probe"]
fn graph_builder_finish_probe() {
    use std::hash::{Hash, Hasher};
    use std::time::{Duration, Instant};

    fn layered_graph(node_count: usize) -> GraphBuilder {
        let layer_count = 8;
        assert_eq!(node_count % layer_count, 0);
        let nodes_per_layer = node_count / layer_count;
        let mut graph = GraphBuilder::default();
        for layer in 0..layer_count {
            for index in 0..nodes_per_layer {
                let id = format!("node_{layer:02}_{index:05}");
                graph.add_node(seed(&id, 1, layer as i32, true));
            }
        }
        for layer in 0..layer_count - 1 {
            for index in 0..nodes_per_layer {
                let from = format!("node_{layer:02}_{index:05}");
                for offset in 0..3 {
                    let target_index = (index * 31 + offset) % nodes_per_layer;
                    let to = format!("node_{:02}_{target_index:05}", layer + 1);
                    graph.add_edge(&from, &to, "derives", 1);
                }
            }
        }
        graph
    }

    fn digest(graph: &crate::project::dependency_graph::AssetDependencyGraph) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for node in &graph.nodes {
            node.id.hash(&mut hasher);
            node.downstream_count.hash(&mut hasher);
            node.mutable.hash(&mut hasher);
            node.mutation_policy.hash(&mut hasher);
        }
        for edge in &graph.edges {
            edge.from.hash(&mut hasher);
            edge.to.hash(&mut hasher);
            edge.relation.hash(&mut hasher);
            edge.count.hash(&mut hasher);
        }
        hasher.finish()
    }

    for node_count in [1_000, 5_000, 10_000] {
        let warmup_until = Instant::now() + Duration::from_millis(250);
        while Instant::now() < warmup_until {
            let _ = layered_graph(node_count).finish(42).unwrap();
        }

        let mut samples_ms = Vec::with_capacity(5);
        let mut expected_digest = None;
        for _ in 0..5 {
            let graph = layered_graph(node_count);
            let started = Instant::now();
            let result = graph.finish(42).unwrap();
            samples_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
            let current_digest = digest(&result);
            assert_eq!(result.edges.len(), node_count * 21 / 8);
            if let Some(expected) = expected_digest {
                assert_eq!(expected, current_digest);
            } else {
                expected_digest = Some(current_digest);
            }
        }
        samples_ms.sort_by(f64::total_cmp);
        println!(
            "asset_graph_finish nodes={node_count} edges={} p50_ms={:.3} digest={:?} samples_ms={samples_ms:?}",
            node_count * 21 / 8,
            samples_ms[samples_ms.len() / 2],
            expected_digest,
        );
    }
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
