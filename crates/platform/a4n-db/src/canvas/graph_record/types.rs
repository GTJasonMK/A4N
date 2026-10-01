pub use a4n_domain::canvas::CanvasGraphRecordDocument;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasGraphRecordSaveResult {
    pub document: CanvasGraphRecordDocument,
    pub node_ids: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasGraphRecordInput {
    pub id: Option<i64>,
    pub expected_version: Option<i32>,
    pub name: Option<String>,
    pub graph_kind: Option<String>,
    pub viewport_json: Option<Value>,
    pub nodes: Vec<CanvasNodeRecordInput>,
    pub edges: Vec<CanvasEdgeRecordInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasNodeRecordInput {
    pub client_id: String,
    pub node_kind: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<i64>,
    pub config_json: Option<Value>,
    pub position_json: Value,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasEdgeRecordInput {
    pub source_client_id: String,
    pub target_client_id: String,
    pub source_port: String,
    pub target_port: String,
    pub role: String,
    pub config_json: Option<Value>,
}
