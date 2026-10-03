use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Persistence projection used by orchestration. ORM rows stay inside `a4n-db`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerationSpecRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_context_kind: Option<String>,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub canvas_text_asset_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: Option<i64>,
    pub status: String,
    pub source_version_hash: String,
    pub params_json: Value,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptArtifactRecord {
    pub id: i64,
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
    pub renderer_version: String,
    pub edited_by_user: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRunTraceTarget {
    pub target_kind: String,
    pub target_table: String,
    pub target_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCompletedGenerationRunInput {
    pub project_id: i64,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_template_names_json: Option<Value>,
    pub prompt_template_hashes_json: Option<Value>,
    pub rendered_messages_json: Option<Value>,
    pub rendered_prompts_json: Option<Value>,
    pub input_variables_json: Option<Value>,
    pub source_refs_json: Option<Value>,
    pub media_refs_json: Option<Value>,
    pub request_json: Option<Value>,
    pub response_summary_json: Option<Value>,
    pub targets: Vec<GenerationRunTraceTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationTaskListCursor {
    pub updated_at_ms: i64,
    pub task_id: String,
}
