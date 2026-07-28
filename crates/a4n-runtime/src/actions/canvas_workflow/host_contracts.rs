use a4n_orchestrator::canvas::{
    CanvasGraphRecordInput as CanvasGraphInput,
    CanvasTextAssetRecordInput as UpsertCanvasTextAssetInput,
};
use serde::{Deserialize, Serialize};

pub use a4n_api_contract::canvas::{CanvasRunSummary, ImportCanvasMediaInput, ImportedCanvasMedia};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetCanvasGraphInput {
    pub project_id: i64,
    pub graph_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveCanvasGraphInput {
    pub project_id: i64,
    pub graph: CanvasGraphInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveCanvasTextAssetInput {
    pub project_id: i64,
    pub text_asset: UpsertCanvasTextAssetInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunCanvasGraphInput {
    pub project_id: i64,
    pub graph_id: i64,
    #[serde(default)]
    pub target_node_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunCanvasNodeInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
}
