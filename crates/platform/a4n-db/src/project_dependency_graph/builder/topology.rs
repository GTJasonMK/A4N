use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::project_dependency_graph::AssetGraphEdge;

pub(super) fn downstream_counts(
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

pub(super) fn find_cycle_hint(
    node_ids: BTreeSet<String>,
    edges: &[AssetGraphEdge],
) -> Option<Vec<String>> {
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
