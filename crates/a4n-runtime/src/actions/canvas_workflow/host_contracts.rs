use crate::actions::{
    image_generation::GeneratedImageSummary, video_generation::GeneratedVideoSummary,
};
use a4n_core::models::media::media_asset;
use a4n_orchestrator::canvas::{
    CanvasGraphRecordInput as CanvasGraphInput,
    CanvasTextAssetRecordInput as UpsertCanvasTextAssetInput,
};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "media_kind", rename_all = "snake_case")]
pub enum ImportedCanvasMedia {
    Image {
        image: GeneratedImageSummary,
        asset: media_asset::Model,
    },
    Video {
        video: GeneratedVideoSummary,
        asset: media_asset::Model,
    },
    Audio {
        asset: media_asset::Model,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasRunSummary {
    pub workflow_run_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub mode: String,
    pub status: String,
    pub error_message: Option<String>,
}
