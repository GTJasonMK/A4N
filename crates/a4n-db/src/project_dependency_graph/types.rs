use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDependencyGraph {
    pub project_id: i64,
    pub generated_at: String,
    pub nodes: Vec<AssetGraphNode>,
    pub edges: Vec<AssetGraphEdge>,
    pub layers: Vec<AssetGraphLayer>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGraphNode {
    pub id: String,
    pub kind: String,
    pub asset_type: String,
    pub label: String,
    pub count: u64,
    pub status: Option<String>,
    pub layer: i32,
    pub chapter_id: Option<i64>,
    pub profile_id: Option<i64>,
    pub record_id: Option<i64>,
    pub editable: bool,
    pub mutable: bool,
    pub direct_downstream_count: u64,
    pub downstream_count: u64,
    pub mutation_policy: String,
    pub needs_visualization: bool,
    pub visualized: bool,
    pub summary: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGraphLayer {
    pub index: i32,
    pub label: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGraphNodeDetail {
    pub node_id: String,
    pub title: String,
    pub kind: String,
    pub asset_type: String,
    pub summary: Option<String>,
    pub records: Vec<AssetLineageRecord>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLineageRecord {
    pub id: String,
    pub label: String,
    pub asset_kind: String,
    pub asset_id: Option<i64>,
    pub status: Option<String>,
    pub origin: String,
    pub source_refs: Vec<AssetLineageRef>,
    pub prompt_sources: Vec<PromptSource>,
    pub media_sources: Vec<AssetLineageRef>,
    pub request_snapshot: Option<Value>,
    pub input_status: String,
    pub generation_runs: Vec<GenerationRunSnapshot>,
    pub generation_inputs: Vec<GenerationInputSource>,
    pub result: Value,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLineageRef {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub asset_type: String,
    pub record_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSource {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub source_record_id: Option<i64>,
    pub edited_by_user: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRunSnapshot {
    pub id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationInputSource {
    pub id: String,
    pub run_id: Option<i64>,
    pub group: String,
    pub kind: String,
    pub label: String,
    pub content_type: String,
    pub text: Option<String>,
    pub value: Option<Value>,
    pub source_record_id: Option<i64>,
}

pub(super) const VIDEO_SEGMENT_PLAN_PROMPT_TYPES: &[&str] = &["frame_segment_plan"];
pub(super) const VIDEO_FRAME_PROMPT_TYPES: &[&str] = &[
    "frame_prompt",
    "first_frame_prompt",
    "chat_video_frame_prompt",
];
pub(super) const CHAPTER_VIDEO_PROMPT_TYPES: &[&str] = &["video_prompt"];
