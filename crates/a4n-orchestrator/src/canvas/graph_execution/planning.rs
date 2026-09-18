use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

use a4n_canvas_contract::CanvasGraphNodeExecutionKind;
use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_db_models::models::canvas::canvas_node;

use crate::canvas::canvas_node_has_persisted_runtime_result;

pub(super) fn canvas_node_kind_is_executable(node_kind: &str) -> bool {
    CanvasGraphNodeExecutionKind::from_node_kind(node_kind).is_ok()
}

pub(super) fn canvas_node_kind_is_output(node_kind: &str) -> bool {
    matches!(
        node_kind,
        "TextOutputNode" | "ImageOutputNode" | "VideoOutputNode" | "AudioOutputNode"
    )
}

pub(super) fn unsupported_canvas_node_execution_error(node_kind: &str) -> String {
    CanvasGraphNodeExecutionKind::from_node_kind(node_kind)
        .expect_err("unsupported helper 只能用于不可执行节点")
}

pub(super) fn runnable_canvas_node_ids(
    document: &CanvasGraphRecordDocument,
    target_node_id: Option<i64>,
) -> Result<Vec<i64>, String> {
    let order = topological_canvas_node_ids(document)?;
    let allowed = target_node_id
        .map(|node_id| upstream_canvas_node_ids(document, node_id))
        .transpose()?;
    let runnable_ids = document
        .nodes
        .iter()
        .filter(|node| is_runnable_canvas_node(node))
        .map(|node| node.id)
        .collect::<HashSet<_>>();
    Ok(order
        .into_iter()
        .filter(|node_id| allowed.as_ref().is_none_or(|ids| ids.contains(node_id)))
        .filter(|node_id| runnable_ids.contains(node_id))
        .filter(|node_id| {
            target_node_id.is_none()
                || document
                    .nodes
                    .iter()
                    .find(|node| node.id == *node_id)
                    .is_none_or(|node| !canvas_node_has_persisted_runtime_result(node))
        })
        .collect())
}

pub(super) fn is_runnable_canvas_node(node: &canvas_node::Model) -> bool {
    canvas_node_kind_is_executable(&node.node_kind)
}

pub(super) fn validate_canvas_output_target(
    document: &CanvasGraphRecordDocument,
    target_node_id: Option<i64>,
) -> Result<(), String> {
    let Some(target_node_id) = target_node_id else {
        return Ok(());
    };
    let target = document
        .nodes
        .iter()
        .find(|node| node.id == target_node_id)
        .ok_or_else(|| format!("画布节点不存在: {target_node_id}"))?;
    if !canvas_node_kind_is_output(&target.node_kind) {
        return Err("运行到此节点只能锚定输出节点".to_string());
    }
    Ok(())
}

fn topological_canvas_node_ids(document: &CanvasGraphRecordDocument) -> Result<Vec<i64>, String> {
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

fn upstream_canvas_node_ids(
    document: &CanvasGraphRecordDocument,
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
