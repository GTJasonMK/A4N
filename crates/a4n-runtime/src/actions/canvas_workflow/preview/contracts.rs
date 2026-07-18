use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewCanvasNodeRequestInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub input_overrides: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasNodeRequestPreview {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub node_kind: String,
    pub status: String,
    pub note: String,
    pub request: Value,
}
