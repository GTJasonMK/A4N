use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub status: String,
    pub note: String,
    pub request: Value,
}
