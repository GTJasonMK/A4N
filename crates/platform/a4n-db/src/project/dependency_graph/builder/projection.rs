use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use a4n_error::AppError;

use super::{
    GraphBuilder,
    topology::{downstream_counts, find_cycle_hint},
};
use crate::project::dependency_graph::{AssetDependencyGraph, AssetGraphLayer, AssetGraphNode};

impl GraphBuilder {
    pub(in crate::project::dependency_graph) fn finish(
        mut self,
        project_id: i64,
    ) -> Result<AssetDependencyGraph, AppError> {
        self.nodes
            .retain(|node_id, seed| node_id == "project" || seed.count > 0);
        let edges = self.retained_edges();
        let node_ids = self.nodes.keys().cloned().collect::<BTreeSet<_>>();
        let downstream = downstream_counts(node_ids.clone(), &edges);
        if let Some(cycle) = find_cycle_hint(node_ids, &edges) {
            self.warnings.push(format!(
                "依赖图存在环，已保留无环层级边但检测到异常节点: {}",
                cycle.join(" -> ")
            ));
        }
        let mut direct_downstream = HashMap::<&str, HashSet<&str>>::new();
        for edge in &edges {
            direct_downstream
                .entry(edge.from.as_str())
                .or_default()
                .insert(edge.to.as_str());
        }

        let mut nodes = self
            .nodes
            .into_values()
            .map(|seed| {
                let direct_downstream_count = direct_downstream
                    .get(seed.id.as_str())
                    .map_or(0, HashSet::len) as u64;
                let downstream_count = downstream.get(&seed.id).copied().unwrap_or_default();
                let mutable = seed.editable && downstream_count == 0;
                let mutation_policy = mutation_policy(seed.editable, downstream_count);
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
            nodes,
            edges,
            layers,
            warnings: self.warnings,
        })
    }
}

fn mutation_policy(editable: bool, downstream_count: u64) -> String {
    if !editable {
        "readonly_aggregate"
    } else if downstream_count == 0 {
        "leaf_editable"
    } else {
        "requires_conflict_resolution"
    }
    .to_string()
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
        7 => "档案与编排",
        8 => "规划",
        9 => "图片素材",
        10 => "边界提示",
        11 => "边界图片",
        12 => "视频提示",
        13 => "最终产物",
        _ => "其他",
    }
}
