use serde_json::Value;

#[derive(Debug, Clone)]
pub(in crate::project_dependency_graph) struct NodeSeed {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) asset_type: String,
    pub(super) label: String,
    pub(super) count: u64,
    pub(super) status: Option<String>,
    pub(super) layer: i32,
    pub(super) chapter_id: Option<i64>,
    pub(super) profile_id: Option<i64>,
    pub(super) record_id: Option<i64>,
    pub(super) editable: bool,
    pub(super) needs_visualization: bool,
    pub(super) visualized: bool,
    pub(super) summary: Option<String>,
    pub(super) details: Value,
}

pub(in crate::project_dependency_graph) struct NodeSeedMetadata {
    pub status: Option<String>,
    pub editable: bool,
    pub needs_visualization: bool,
    pub visualized: bool,
    pub summary: Option<String>,
    pub details: Value,
}

impl NodeSeed {
    pub(in crate::project_dependency_graph) fn profile_id(mut self, profile_id: i64) -> Self {
        self.profile_id = Some(profile_id);
        self
    }

    pub(in crate::project_dependency_graph) fn record_id(mut self, record_id: i64) -> Self {
        self.record_id = Some(record_id);
        self
    }
}

pub(in crate::project_dependency_graph) fn node(
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
