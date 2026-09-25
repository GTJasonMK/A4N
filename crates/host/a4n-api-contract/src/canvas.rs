use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{image::GeneratedImageSummary, video::GeneratedVideoSummary};

pub type CanvasJsonObject = BTreeMap<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasGraphSummary {
    pub id: i64,
    pub name: String,
    pub graph_kind: String,
    pub version: i32,
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
pub struct CanvasGraphSaveResult {
    #[serde(flatten)]
    pub document: CanvasGraphDocument,
    pub node_ids: BTreeMap<String, i64>,
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
    #[schemars(range(min = 1))]
    pub expected_version: Option<i32>,
    pub graph: CanvasGraphInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadCanvasGraphInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub graph_id: i64,
    pub expected_version: Option<i32>,
    /// None reads a paged index; IDs read those nodes and their incident edges.
    pub node_ids: Option<Vec<i64>>,
    #[serde(default)]
    pub offset: u32,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CanvasNodeIndex {
    pub id: i64,
    pub node_kind: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "view", rename_all = "snake_case")]
pub enum CanvasGraphProjection {
    Index {
        nodes: Vec<CanvasNodeIndex>,
        total: usize,
        next_offset: Option<u32>,
    },
    Nodes {
        nodes: Vec<CanvasNodeSummary>,
        edges: Vec<CanvasEdgeSummary>,
        adjacent_nodes: Vec<CanvasNodeIndex>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CanvasGraphReadResult {
    pub resource: crate::agent::AgentResourceRef,
    pub projection: CanvasGraphProjection,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanvasGraphEditOperation {
    SetNodeConfig {
        node_id: i64,
        config_json: CanvasJsonObject,
    },
    Connect {
        edge: CanvasEdgeInput,
    },
    Disconnect {
        edge_id: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditCanvasGraphInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub graph_id: i64,
    #[schemars(range(min = 1))]
    pub expected_version: i32,
    #[schemars(length(min = 1, max = 20))]
    pub operations: Vec<CanvasGraphEditOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CanvasGraphEditResult {
    pub resource: crate::agent::AgentResourceRef,
    pub changed: bool,
    pub changed_nodes: Vec<CanvasNodeSummary>,
    pub added_edges: Vec<CanvasEdgeSummary>,
    pub removed_edge_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SaveCanvasTextAssetInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub text_asset: CanvasTextAssetInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanvasMediaAssetSummary {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub media_kind: String,
    pub origin: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub file_size: Option<i64>,
    pub checksum: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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

/// Typed input passed from Canvas preview preparation to the video owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateCanvasVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    #[serde(default)]
    pub workflow_run_id: Option<i64>,
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub config_id: Option<i64>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub duration_secs: Option<i32>,
    #[serde(default)]
    pub fps: Option<i32>,
    #[serde(default)]
    pub first_frame_image_id: Option<i64>,
    #[serde(default)]
    pub last_frame_image_id: Option<i64>,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
    #[serde(default)]
    pub source_node_ids: Vec<i64>,
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
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub data_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportedCanvasMedia {
    pub media_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<GeneratedImageSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<GeneratedVideoSummary>,
    pub asset: CanvasMediaAssetSummary,
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

/// Host-facing Canvas read boundary.
///
/// Graph validation, project permissions, version checks and persistence stay
/// with the implementing application. Runtime only routes API requests to
/// this port and does not depend on the concrete orchestrator facade.
#[async_trait::async_trait]
pub trait CanvasReadApplicationPort: Send + Sync {
    async fn get_graph(
        &self,
        user_id: i64,
        input: GetCanvasGraphInput,
    ) -> Result<CanvasGraphDocument, a4n_error::AppError>;

    async fn read_graph(
        &self,
        user_id: i64,
        input: ReadCanvasGraphInput,
    ) -> Result<CanvasGraphReadResult, a4n_error::AppError>;
}

/// Host-facing Canvas mutation boundary.
#[async_trait::async_trait]
pub trait CanvasMutationApplicationPort: Send + Sync {
    async fn save_graph(
        &self,
        user_id: i64,
        input: SaveCanvasGraphInput,
    ) -> Result<CanvasGraphSaveResult, a4n_error::AppError>;

    async fn edit_graph(
        &self,
        user_id: i64,
        input: EditCanvasGraphInput,
    ) -> Result<CanvasGraphEditResult, a4n_error::AppError>;

    async fn save_text_asset(
        &self,
        user_id: i64,
        input: SaveCanvasTextAssetInput,
    ) -> Result<CanvasTextAssetSummary, a4n_error::AppError>;
}

fn canvas_port(
    id: &str,
    label: &str,
    data_type: &str,
    direction: &str,
    required: bool,
    multiple: bool,
) -> CanvasPortContract {
    CanvasPortContract {
        id: id.to_string(),
        label: label.to_string(),
        data_type: data_type.to_string(),
        direction: direction.to_string(),
        required,
        multiple,
    }
}

fn canvas_input(
    id: &str,
    label: &str,
    data_type: &str,
    required: bool,
    multiple: bool,
) -> CanvasPortContract {
    canvas_port(id, label, data_type, "input", required, multiple)
}

fn canvas_output(id: &str, label: &str, data_type: &str) -> CanvasPortContract {
    canvas_port(id, label, data_type, "output", false, true)
}

fn canvas_node(
    kind: &str,
    label: &str,
    category: &str,
    runnable: bool,
    inputs: Vec<CanvasPortContract>,
    outputs: Vec<CanvasPortContract>,
) -> CanvasNodeContract {
    CanvasNodeContract {
        kind: kind.to_string(),
        label: label.to_string(),
        category: category.to_string(),
        runnable,
        inputs,
        outputs,
    }
}

/// Stable Canvas node and port contracts consumed by all hosts.
pub fn canvas_node_contracts() -> Vec<CanvasNodeContract> {
    vec![
        canvas_node(
            "TextInputNode",
            "文本输入",
            "input",
            false,
            vec![],
            vec![canvas_output("text", "文本", "text")],
        ),
        canvas_node(
            "ImageInputNode",
            "图片输入",
            "input",
            false,
            vec![],
            vec![canvas_output("image", "图片", "image")],
        ),
        canvas_node(
            "VideoInputNode",
            "视频输入",
            "input",
            false,
            vec![],
            vec![canvas_output("video", "视频", "video")],
        ),
        canvas_node(
            "AudioInputNode",
            "音频输入",
            "input",
            false,
            vec![],
            vec![canvas_output("audio", "音频", "audio")],
        ),
        canvas_node(
            "RenderTextNode",
            "文本模板",
            "operation",
            true,
            vec![canvas_input("input", "文本", "text", false, true)],
            vec![canvas_output("text", "文本", "text")],
        ),
        canvas_node(
            "LlmTextNode",
            "LLM文本处理",
            "operation",
            true,
            vec![canvas_input("input", "文本", "text", true, true)],
            vec![canvas_output("text", "文本", "text")],
        ),
        canvas_node(
            "GenerateImageNode",
            "生成图片",
            "operation",
            true,
            vec![canvas_input("text", "文本", "text", false, true)],
            vec![canvas_output("image", "图片", "image")],
        ),
        canvas_node(
            "GenerateVideoNode",
            "生成视频",
            "operation",
            true,
            vec![
                canvas_input("text", "文本", "text", false, true),
                canvas_input("first_frame", "首帧", "image", false, false),
                canvas_input("end_frame", "尾帧", "image", false, false),
                canvas_input("reference_image", "参考图", "image", false, true),
            ],
            vec![canvas_output("video", "视频", "video")],
        ),
        canvas_node(
            "ExtractFrameNode",
            "抽取帧",
            "operation",
            true,
            vec![canvas_input("video", "视频", "video", true, false)],
            vec![canvas_output("image", "帧", "image")],
        ),
        canvas_node(
            "TrimVideoNode",
            "裁剪视频",
            "operation",
            true,
            vec![canvas_input("video", "视频", "video", true, false)],
            vec![canvas_output("video", "视频", "video")],
        ),
        canvas_node(
            "AssembleVideoNode",
            "组装视频",
            "operation",
            true,
            vec![canvas_input("clip", "片段", "video", true, true)],
            vec![canvas_output("video", "视频", "video")],
        ),
        canvas_node(
            "TextProbeNode",
            "文本探针",
            "probe",
            true,
            vec![canvas_input("text", "文本", "text", true, false)],
            vec![canvas_output("text", "文本", "text")],
        ),
        canvas_node(
            "ImageProbeNode",
            "图片探针",
            "probe",
            true,
            vec![canvas_input("image", "图片", "image", true, false)],
            vec![canvas_output("image", "图片", "image")],
        ),
        canvas_node(
            "VideoProbeNode",
            "视频探针",
            "probe",
            true,
            vec![canvas_input("video", "视频", "video", true, false)],
            vec![canvas_output("video", "视频", "video")],
        ),
        canvas_node(
            "AudioProbeNode",
            "音频探针",
            "probe",
            true,
            vec![canvas_input("audio", "音频", "audio", true, false)],
            vec![canvas_output("audio", "音频", "audio")],
        ),
        canvas_node(
            "TextOutputNode",
            "文本输出",
            "output",
            true,
            vec![canvas_input("text", "文本", "text", true, false)],
            vec![],
        ),
        canvas_node(
            "ImageOutputNode",
            "图片输出",
            "output",
            true,
            vec![canvas_input("image", "图片", "image", true, false)],
            vec![canvas_output("image", "图片", "image")],
        ),
        canvas_node(
            "VideoOutputNode",
            "视频输出",
            "output",
            true,
            vec![canvas_input("video", "视频", "video", true, false)],
            vec![],
        ),
        canvas_node(
            "AudioOutputNode",
            "音频输出",
            "output",
            true,
            vec![canvas_input("audio", "音频", "audio", true, false)],
            vec![],
        ),
    ]
}
