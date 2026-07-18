use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn topological_canvas_node_ids(
    document: &CanvasGraphDocument,
) -> Result<Vec<i64>, String> {
    let node_ids = document
        .nodes
        .iter()
        .map(|node| node.id)
        .collect::<BTreeSet<_>>();
    let mut indegree = document
        .nodes
        .iter()
        .map(|node| (node.id, 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = document
        .nodes
        .iter()
        .map(|node| (node.id, Vec::new()))
        .collect::<BTreeMap<_, _>>();

    for edge in &document.edges {
        if !node_ids.contains(&edge.source_node_id) {
            return Err(format!(
                "画布连线引用不存在的源节点: {}",
                edge.source_node_id
            ));
        }
        if !node_ids.contains(&edge.target_node_id) {
            return Err(format!(
                "画布连线引用不存在的目标节点: {}",
                edge.target_node_id
            ));
        }
        *indegree.entry(edge.target_node_id).or_default() += 1;
        outgoing
            .entry(edge.source_node_id)
            .or_default()
            .push(edge.target_node_id);
    }

    let mut queue = indegree
        .iter()
        .filter_map(|(node_id, degree)| (*degree == 0).then_some(*node_id))
        .collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(document.nodes.len());
    while let Some(node_id) = queue.pop_front() {
        order.push(node_id);
        for target_id in outgoing.get(&node_id).into_iter().flatten() {
            if let Some(degree) = indegree.get_mut(target_id) {
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(*target_id);
                }
            }
        }
    }
    if order.len() != document.nodes.len() {
        return Err("画布暂只支持有向无环图，当前连线存在环路".to_string());
    }
    Ok(order)
}

pub(super) fn upstream_canvas_node_ids(
    document: &CanvasGraphDocument,
    target_node_id: i64,
) -> Result<BTreeSet<i64>, String> {
    if !document.nodes.iter().any(|node| node.id == target_node_id) {
        return Err(format!("画布节点不存在: {target_node_id}"));
    }
    let mut incoming = BTreeMap::<i64, Vec<i64>>::new();
    for edge in &document.edges {
        incoming
            .entry(edge.target_node_id)
            .or_default()
            .push(edge.source_node_id);
    }
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([target_node_id]);
    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id) {
            continue;
        }
        for source_id in incoming.get(&node_id).into_iter().flatten() {
            queue.push_back(*source_id);
        }
    }
    Ok(visited)
}
