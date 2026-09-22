mod projection;
mod seed;
mod topology;

use std::collections::BTreeMap;

use self::seed::NodeSeed;
use super::AssetGraphEdge;

pub(super) use self::seed::{NodeSeedMetadata, node};

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

    fn retained_edges(&self) -> Vec<AssetGraphEdge> {
        self.edge_counts
            .iter()
            .filter(|((from, to, _), _)| {
                self.nodes.contains_key(from) && self.nodes.contains_key(to)
            })
            .map(|((from, to, relation), count)| AssetGraphEdge {
                from: from.clone(),
                to: to.clone(),
                relation: relation.clone(),
                count: *count,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
