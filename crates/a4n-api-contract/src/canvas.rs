use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type CanvasJsonObject = BTreeMap<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasGraphSummary {
    pub id: i64,
    pub name: String,
    pub graph_kind: String,
    pub viewport_json: Option<CanvasJsonObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasNodeSummary {
    pub id: i64,
    pub node_kind: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<i64>,
    pub config_json: CanvasJsonObject,
    pub position_json: CanvasJsonObject,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasEdgeSummary {
    pub id: i64,
    pub source_node_id: i64,
    pub target_node_id: i64,
    pub source_port: String,
    pub target_port: String,
    pub role: String,
    pub config_json: CanvasJsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasGraphDocument {
    pub graph: CanvasGraphSummary,
    pub nodes: Vec<CanvasNodeSummary>,
    pub edges: Vec<CanvasEdgeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanvasNodeInput {
    pub client_id: String,
    pub node_kind: String,
    pub ref_type: Option<String>,
    #[schemars(range(min = 1))]
    pub ref_id: Option<i64>,
    pub config_json: CanvasJsonObject,
    pub position_json: CanvasJsonObject,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanvasEdgeInput {
    pub source_client_id: String,
    pub target_client_id: String,
    pub source_port: String,
    pub target_port: String,
    pub role: String,
    pub config_json: CanvasJsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanvasGraphInput {
    #[schemars(range(min = 1))]
    pub id: Option<i64>,
    pub name: String,
    pub graph_kind: String,
    pub viewport_json: Option<CanvasJsonObject>,
    pub nodes: Vec<CanvasNodeInput>,
    pub edges: Vec<CanvasEdgeInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanvasTextAssetInput {
    #[schemars(range(min = 1))]
    pub id: Option<i64>,
    pub title: String,
    pub text_kind: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasTextAssetSummary {
    pub id: i64,
    pub text_kind: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GetCanvasGraphInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub graph_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GetCanvasNodeContractsInput {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SaveCanvasGraphInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub graph: CanvasGraphInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SaveCanvasTextAssetInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub text_asset: CanvasTextAssetInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreviewCanvasNodeRequestInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub graph_id: i64,
    #[schemars(range(min = 1))]
    pub node_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunCanvasGraphInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub graph_id: i64,
    #[schemars(range(min = 1))]
    pub target_node_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunCanvasNodeInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub graph_id: i64,
    #[schemars(range(min = 1))]
    pub node_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasPortContract {
    pub id: String,
    pub label: String,
    pub data_type: String,
    pub direction: String,
    pub required: bool,
    pub multiple: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasNodeContract {
    pub kind: String,
    pub label: String,
    pub category: String,
    pub runnable: bool,
    pub inputs: Vec<CanvasPortContract>,
    pub outputs: Vec<CanvasPortContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportCanvasMediaInput {
    pub project_id: i64,
    pub media_kind: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub data_base64: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportedCanvasMedia {
    pub media_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<Value>,
    pub asset: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasRunSummary {
    pub workflow_run_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub mode: String,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasNodeRequestPreview {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub node_kind: String,
    pub note: String,
    pub request: Value,
}
