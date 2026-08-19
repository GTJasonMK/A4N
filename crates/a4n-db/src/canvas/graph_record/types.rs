use a4n_core::models::canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasGraphRecordDocument {
    pub graph: canvas_graph::Model,
    pub nodes: Vec<canvas_node::Model>,
    pub edges: Vec<canvas_edge::Model>,
    pub text_assets: Vec<canvas_text_asset::Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasGraphRecordInput {
    pub id: Option<i64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasTextAssetRecordInput {
    pub id: Option<i64>,
    pub title: String,
    pub text_kind: String,
    pub content: String,
}
