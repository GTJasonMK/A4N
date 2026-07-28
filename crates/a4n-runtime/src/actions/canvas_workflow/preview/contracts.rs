use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use a4n_api_contract::canvas::CanvasNodeRequestPreview;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewCanvasNodeRequestInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub input_overrides: Option<Value>,
}
