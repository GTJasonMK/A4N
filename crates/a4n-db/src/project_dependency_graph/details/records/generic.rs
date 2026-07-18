use crate::project_dependency_graph::{
    AssetDependencyGraph, AssetGraphNode, AssetLineageRecord, AssetLineageRef, PromptSource,
};

pub(in crate::project_dependency_graph) fn structural_source_refs_for_node(
    graph: &AssetDependencyGraph,
    node_id: &str,
) -> Vec<AssetLineageRef> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.to == node_id && edge.count > 0)
        .filter_map(|edge| {
            graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.from)
                .map(|source| {
                    lineage_ref(
                        &source.id,
                        &source.label,
                        &source.kind,
                        &source.asset_type,
                        source.record_id,
                    )
                })
        })
        .collect()
}

pub(in crate::project_dependency_graph) fn generic_node_detail_records(
    node: &AssetGraphNode,
    source_refs: Vec<AssetLineageRef>,
) -> Vec<AssetLineageRecord> {
    vec![AssetLineageRecord {
        id: node.id.clone(),
        label: node.label.clone(),
        asset_kind: node.asset_type.clone(),
        asset_id: node.record_id,
        status: node.status.clone(),
        origin: if node.kind == "text" {
            "derived_text"
        } else {
            "unknown"
        }
        .to_string(),
        source_refs,
        prompt_sources: Vec::new(),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: node.details.clone(),
        warnings: Vec::new(),
    }]
}

pub(in crate::project_dependency_graph) fn lineage_ref(
    id: &str,
    label: &str,
    kind: &str,
    asset_type: &str,
    record_id: Option<i64>,
) -> AssetLineageRef {
    AssetLineageRef {
        id: id.to_string(),
        label: label.to_string(),
        kind: kind.to_string(),
        asset_type: asset_type.to_string(),
        record_id,
    }
}

pub(in crate::project_dependency_graph) fn prompt_source(
    id: &str,
    kind: &str,
    text: Option<&str>,
    source_record_id: Option<i64>,
    edited_by_user: Option<bool>,
) -> Option<PromptSource> {
    let text = text?.trim();
    if text.is_empty() {
        return None;
    }
    Some(PromptSource {
        id: id.to_string(),
        kind: kind.to_string(),
        text: text.to_string(),
        source_record_id,
        edited_by_user,
    })
}

pub(in crate::project_dependency_graph) fn optional_prompt_sources(
    items: Vec<Option<PromptSource>>,
) -> Vec<PromptSource> {
    items.into_iter().flatten().collect()
}
