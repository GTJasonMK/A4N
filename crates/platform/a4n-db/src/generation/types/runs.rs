use a4n_domain::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::checkpoint::GenerationRunCheckpointContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordStartedGenerationRunInput {
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
    pub targets: Vec<GenerationRunTraceTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCompletedGenerationRunWithCheckpointInput {
    pub run: RecordCompletedGenerationRunInput,
    pub checkpoint: GenerationRunCheckpointContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachGenerationRunsToWorkflowInput {
    pub target_table: String,
    pub target_id: i64,
    pub workflow_run_id: i64,
    pub call_key: Option<String>,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub artifact_refs_json: Option<Value>,
}

pub(in crate::generation) struct CreateGenerationRunInput {
    pub(in crate::generation) project_id: i64,
    pub(in crate::generation) root_project_id: i64,
    pub(in crate::generation) workflow: String,
    pub(in crate::generation) step_key: String,
    pub(in crate::generation) title: String,
    pub(in crate::generation) model_kind: String,
    pub(in crate::generation) provider: Option<String>,
    pub(in crate::generation) model: Option<String>,
    pub(in crate::generation) checkpoint: GenerationRunCheckpointContext,
    pub(in crate::generation) prompt_template_names_json: Option<Value>,
    pub(in crate::generation) prompt_template_hashes_json: Option<Value>,
    pub(in crate::generation) rendered_messages_json: Option<Value>,
    pub(in crate::generation) rendered_prompts_json: Option<Value>,
    pub(in crate::generation) input_variables_json: Option<Value>,
    pub(in crate::generation) source_refs_json: Option<Value>,
    pub(in crate::generation) media_refs_json: Option<Value>,
    pub(in crate::generation) request_json: Option<Value>,
}
