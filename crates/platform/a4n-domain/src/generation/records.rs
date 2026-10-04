use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct GenerationTaskCheckpointRecord {
    pub task_id: String,
    pub business_key: String,
    pub attempt: i32,
    pub requested_by_user_id: i64,
    pub task_kind: String,
    pub delivery_mode: String,
    pub batch_policy: Option<String>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub generation_run_id: Option<i64>,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub target_revision: Option<String>,
    pub root_task_id: String,
    pub parent_task_id: Option<String>,
    pub item_key: Option<String>,
    pub retry_of_task_id: Option<String>,
    pub status: String,
    pub checkpoint_revision: i64,
    pub covers_through_sequence: i64,
    pub reserved_through_sequence: i64,
    pub public_snapshot_json: Option<Value>,
    pub public_snapshot_sha256: Option<String>,
    pub step_key: Option<String>,
    pub call_key: Option<String>,
    pub provider_kind: Option<String>,
    pub provider_model: Option<String>,
    pub provider_job_id: Option<String>,
    pub provider_idempotency_key_sha256: Option<String>,
    pub resume_metadata_version: Option<i32>,
    pub execution_checkpoint_json: Option<Value>,
    pub execution_checkpoint_sha256: Option<String>,
    pub input_fingerprint_sha256: String,
    pub result_kind: Option<String>,
    pub result_id: Option<String>,
    pub result_revision: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}

impl GenerationTaskCheckpointRecord {
    pub fn summary(&self) -> GenerationTaskCheckpointSummaryRecord {
        GenerationTaskCheckpointSummaryRecord {
            task_id: self.task_id.clone(),
            attempt: self.attempt,
            requested_by_user_id: self.requested_by_user_id,
            task_kind: self.task_kind.clone(),
            delivery_mode: self.delivery_mode.clone(),
            batch_policy: self.batch_policy.clone(),
            project_id: self.project_id,
            root_project_id: self.root_project_id,
            workflow_run_id: self.workflow_run_id,
            generation_run_id: self.generation_run_id,
            target_kind: self.target_kind.clone(),
            target_id: self.target_id.clone(),
            target_revision: self.target_revision.clone(),
            root_task_id: self.root_task_id.clone(),
            parent_task_id: self.parent_task_id.clone(),
            item_key: self.item_key.clone(),
            retry_of_task_id: self.retry_of_task_id.clone(),
            status: self.status.clone(),
            covers_through_sequence: self.covers_through_sequence,
            public_snapshot_json: self.public_snapshot_json.clone(),
            public_snapshot_sha256: self.public_snapshot_sha256.clone(),
            result_kind: self.result_kind.clone(),
            result_id: self.result_id.clone(),
            result_revision: self.result_revision.clone(),
            expires_at: self.expires_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenerationTaskCheckpointSummaryRecord {
    pub task_id: String,
    pub attempt: i32,
    pub requested_by_user_id: i64,
    pub task_kind: String,
    pub delivery_mode: String,
    pub batch_policy: Option<String>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub generation_run_id: Option<i64>,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub target_revision: Option<String>,
    pub root_task_id: String,
    pub parent_task_id: Option<String>,
    pub item_key: Option<String>,
    pub retry_of_task_id: Option<String>,
    pub status: String,
    pub covers_through_sequence: i64,
    pub public_snapshot_json: Option<Value>,
    pub public_snapshot_sha256: Option<String>,
    pub result_kind: Option<String>,
    pub result_id: Option<String>,
    pub result_revision: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
}

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
