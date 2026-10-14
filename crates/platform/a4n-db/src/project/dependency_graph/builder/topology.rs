use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use crate::project::dependency_graph::AssetGraphEdge;

pub(super) fn downstream_counts(
    node_ids: BTreeSet<String>,
    edges: &[AssetGraphEdge],
) -> BTreeMap<String, u64> {
    let node_ids = node_ids.into_iter().collect::<Vec<_>>();
    let index_by_id = node_ids
        .iter()
        .enumerate()
        .map(|(index, node_id)| (node_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut adjacency = vec![Vec::<usize>::new(); node_ids.len()];
    for edge in edges {
        if let (Some(&from), Some(&to)) = (
            index_by_id.get(edge.from.as_str()),
            index_by_id.get(edge.to.as_str()),
        ) {
            adjacency[from].push(to);
        }
    }
    drop(index_by_id);
    for children in &mut adjacency {
        children.sort_unstable();
        children.dedup();
    }

    let mut visited_at = vec![0; node_ids.len()];
    let mut queue = Vec::<usize>::new();
    let mut counts = BTreeMap::new();
    for (source_index, node_id) in node_ids.into_iter().enumerate() {
        let generation = source_index + 1;
        let mut cursor = 0;
        let mut count = 0_u64;
        queue.clear();
        queue.extend_from_slice(&adjacency[source_index]);
        while cursor < queue.len() {
            let next = queue[cursor];
            cursor += 1;
            if visited_at[next] == generation {
                continue;
            }
            visited_at[next] = generation;
            count += 1;
            queue.extend_from_slice(&adjacency[next]);
        }
        counts.insert(node_id, count);
    }
    counts
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
