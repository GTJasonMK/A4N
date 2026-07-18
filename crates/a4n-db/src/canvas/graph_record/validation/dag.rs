use std::collections::BTreeMap;

use crate::canvas::CanvasDbError;

use super::super::types::{CanvasGraphRecordDocument, CanvasGraphRecordInput};

pub fn validate_canvas_record_document_is_dag(
    document: &CanvasGraphRecordDocument,
) -> Result<(), CanvasDbError> {
    let mut indegree = BTreeMap::<i64, usize>::new();
    let mut outgoing = BTreeMap::<i64, Vec<i64>>::new();
    for node in &document.nodes {
        indegree.insert(node.id, 0);
        outgoing.insert(node.id, Vec::new());
    }
    for edge in &document.edges {
        if let Some(value) = indegree.get_mut(&edge.target_node_id) {
            *value += 1;
        }
        outgoing
            .entry(edge.source_node_id)
            .or_default()
            .push(edge.target_node_id);
    }
    validate_canvas_record_dag_from_degrees(indegree, outgoing)
}

pub(super) fn validate_canvas_graph_record_is_dag(
    input: &CanvasGraphRecordInput,
) -> Result<(), CanvasDbError> {
    let mut indegree = BTreeMap::<String, usize>::new();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for node in &input.nodes {
        let client_id = node.client_id.trim().to_string();
        indegree.insert(client_id.clone(), 0);
        outgoing.insert(client_id, Vec::new());
    }
    for edge in &input.edges {
        let source = edge.source_client_id.trim().to_string();
        let target = edge.target_client_id.trim().to_string();
        if let Some(value) = indegree.get_mut(&target) {
            *value += 1;
        }
        outgoing.entry(source).or_default().push(target);
    }
    validate_canvas_record_dag_from_degrees(indegree, outgoing)
}

fn validate_canvas_record_dag_from_degrees<T>(
    mut indegree: BTreeMap<T, usize>,
    outgoing: BTreeMap<T, Vec<T>>,
) -> Result<(), CanvasDbError>
where
    T: Clone + Ord,
{
    let mut queue = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(node, _)| node.clone())
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    while let Some(node) = queue.pop() {
        visited += 1;
        for target in outgoing.get(&node).into_iter().flatten() {
            if let Some(degree) = indegree.get_mut(target) {
                *degree -= 1;
                if *degree == 0 {
                    queue.push(target.clone());
                }
            }
        }
    }
    if visited != indegree.len() {
        return Err(CanvasDbError::InvalidArgument(
            "画布暂只支持有向无环图，当前连线存在环路".to_string(),
        ));
    }
    Ok(())
}
