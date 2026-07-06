use super::{AssetDependencyGraph, AssetGraphEdge, AssetGraphLayer, AssetGraphNode};

use a4n_core::error::AppError;
use chrono::Utc;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone)]
pub(super) struct NodeSeed {
    id: String,
    kind: String,
    asset_type: String,
    label: String,
    count: u64,
    status: Option<String>,
    layer: i32,
    chapter_id: Option<i64>,
    profile_id: Option<i64>,
    record_id: Option<i64>,
    editable: bool,
    needs_visualization: bool,
    visualized: bool,
    summary: Option<String>,
    details: Value,
}

pub(super) struct NodeSeedMetadata {
    pub status: Option<String>,
    pub editable: bool,
    pub needs_visualization: bool,
    pub visualized: bool,
    pub summary: Option<String>,
    pub details: Value,
}

#[derive(Debug, Default)]
pub(super) struct GraphBuilder {
    nodes: BTreeMap<String, NodeSeed>,
    edge_counts: BTreeMap<(String, String, String), u64>,
    warnings: Vec<String>,
}

impl GraphBuilder {
    pub(super) fn add_node(&mut self, seed: NodeSeed) {
        self.nodes.entry(seed.id.clone()).or_insert(seed);
    }

    pub(super) fn add_edge(&mut self, from: &str, to: &str, relation: &str, count: u64) {
        if count == 0 {
            return;
        }
        if from == to {
            self.warnings
                .push(format!("已跳过自循环依赖边: {from} -> {to}"));
            return;
        }
        let Some(from_node) = self.nodes.get(from) else {
            self.warnings
                .push(format!("依赖边缺少上游节点，已跳过: {from} -> {to}"));
            return;
        };
        let Some(to_node) = self.nodes.get(to) else {
            self.warnings
                .push(format!("依赖边缺少下游节点，已跳过: {from} -> {to}"));
            return;
        };
        if from_node.layer >= to_node.layer {
            self.warnings
                .push(format!("依赖边不符合拓扑层级，已跳过: {from} -> {to}"));
            return;
        }
        *self
            .edge_counts
            .entry((from.to_string(), to.to_string(), relation.to_string()))
            .or_insert(0) += count;
    }

    pub(super) fn node_count(&self, node_id: &str) -> u64 {
        self.nodes
            .get(node_id)
            .map(|node| node.count)
            .unwrap_or_default()
    }

    pub(super) fn finish(mut self, project_id: i64) -> Result<AssetDependencyGraph, AppError> {
        self.nodes
            .retain(|node_id, seed| node_id == "project" || seed.count > 0);
        self.edge_counts.retain(|(from, to, _), _| {
            self.nodes.contains_key(from) && self.nodes.contains_key(to)
        });

        let edges = self
            .edge_counts
            .into_iter()
            .map(|((from, to, relation), count)| AssetGraphEdge {
                from,
                to,
                relation,
                count,
            })
            .collect::<Vec<_>>();
        let downstream = downstream_counts(self.nodes.keys().cloned().collect(), &edges);
        if let Some(cycle) = find_cycle_hint(self.nodes.keys().cloned().collect(), &edges) {
            self.warnings.push(format!(
                "依赖图存在环，已保留无环层级边但检测到异常节点: {}",
                cycle.join(" -> ")
            ));
        }

        let mut nodes = self
            .nodes
            .into_values()
            .map(|seed| {
                let direct_downstream_count = edges
                    .iter()
                    .filter(|edge| edge.from == seed.id)
                    .map(|edge| edge.to.as_str())
                    .collect::<BTreeSet<_>>()
                    .len() as u64;
                let downstream_count = downstream.get(&seed.id).copied().unwrap_or_default();
                let mutable = seed.editable && downstream_count == 0;
                let mutation_policy = if !seed.editable {
                    "readonly_aggregate".to_string()
                } else if downstream_count == 0 {
                    "leaf_editable".to_string()
                } else {
                    "requires_conflict_resolution".to_string()
                };
                AssetGraphNode {
                    id: seed.id,
                    kind: seed.kind,
                    asset_type: seed.asset_type,
                    label: seed.label,
                    count: seed.count,
                    status: seed.status,
                    layer: seed.layer,
                    chapter_id: seed.chapter_id,
                    profile_id: seed.profile_id,
                    record_id: seed.record_id,
                    editable: seed.editable,
                    mutable,
                    direct_downstream_count,
                    downstream_count,
                    mutation_policy,
                    needs_visualization: seed.needs_visualization,
                    visualized: seed.visualized,
                    summary: seed.summary,
                    details: seed.details,
                }
            })
            .collect::<Vec<_>>();
        nodes.sort_by(|a, b| (a.layer, a.id.as_str()).cmp(&(b.layer, b.id.as_str())));

        let mut layer_map = BTreeMap::<i32, Vec<String>>::new();
        for node in &nodes {
            layer_map
                .entry(node.layer)
                .or_default()
                .push(node.id.clone());
        }
        let layers = layer_map
            .into_iter()
            .map(|(index, node_ids)| AssetGraphLayer {
                index,
                label: layer_label(index).to_string(),
                node_ids,
            })
            .collect();

        Ok(AssetDependencyGraph {
            project_id,
            generated_at: Utc::now().to_rfc3339(),
            nodes,
            edges,
            layers,
            warnings: self.warnings,
        })
    }
}

impl NodeSeed {
    pub(super) fn profile_id(mut self, profile_id: i64) -> Self {
        self.profile_id = Some(profile_id);
        self
    }

    pub(super) fn record_id(mut self, record_id: i64) -> Self {
        self.record_id = Some(record_id);
        self
    }
}

pub(super) fn node(
    id: &str,
    kind: &str,
    asset_type: &str,
    label: &str,
    count: u64,
    layer: i32,
    metadata: NodeSeedMetadata,
) -> NodeSeed {
    NodeSeed {
        id: id.to_string(),
        kind: kind.to_string(),
        asset_type: asset_type.to_string(),
        label: label.to_string(),
        count,
        status: metadata.status,
        layer,
        chapter_id: None,
        profile_id: None,
        record_id: None,
        editable: metadata.editable,
        needs_visualization: metadata.needs_visualization,
        visualized: metadata.visualized,
        summary: metadata.summary,
        details: metadata.details,
    }
}

fn downstream_counts(
    node_ids: BTreeSet<String>,
    edges: &[AssetGraphEdge],
) -> BTreeMap<String, u64> {
    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    for edge in edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .insert(edge.to.clone());
    }
    node_ids
        .into_iter()
        .map(|node_id| {
            let mut seen = BTreeSet::<String>::new();
            let mut queue = VecDeque::<String>::new();
            if let Some(next) = adjacency.get(&node_id) {
                queue.extend(next.iter().cloned());
            }
            while let Some(next) = queue.pop_front() {
                if !seen.insert(next.clone()) {
                    continue;
                }
                if let Some(children) = adjacency.get(&next) {
                    queue.extend(children.iter().cloned());
                }
            }
            (node_id, seen.len() as u64)
        })
        .collect()
}

fn find_cycle_hint(node_ids: BTreeSet<String>, edges: &[AssetGraphEdge]) -> Option<Vec<String>> {
    let mut indegree = node_ids
        .iter()
        .map(|node| (node.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    for edge in edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        if let Some(value) = indegree.get_mut(&edge.to) {
            *value += 1;
        }
    }
    let mut queue = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
        .collect::<VecDeque<_>>();
    let mut visited = BTreeSet::new();
    while let Some(node) = queue.pop_front() {
        visited.insert(node.clone());
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                let Some(degree) = indegree.get_mut(child) else {
                    continue;
                };
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(child.clone());
                }
            }
        }
    }
    if visited.len() == indegree.len() {
        return None;
    }
    Some(
        indegree
            .into_iter()
            .filter_map(|(node, degree)| (degree > 0).then_some(node))
            .take(8)
            .collect(),
    )
}

fn layer_label(index: i32) -> &'static str {
    match index {
        0 => "项目",
        1 => "灵感",
        2 => "蓝图",
        3 => "分部",
        4 => "章节大纲",
        5 => "正文",
        6 => "分析",
        7 => "档案",
        8 => "规划",
        9 => "图片素材",
        10 => "边界提示",
        11 => "边界图片",
        12 => "视频提示",
        13 => "最终产物",
        _ => "其他",
    }
}
