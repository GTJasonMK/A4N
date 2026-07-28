use super::generation_tasks::GenerationStartRequestMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationAgentPermissions {
    pub read_project_detail: bool,
    pub read_blueprint: bool,
    pub read_outlines: bool,
    pub read_character_profiles: bool,
    pub read_foreshadowing: bool,
    #[serde(default)]
    pub automatic_writeback: bool,
}

impl Default for OptimizationAgentPermissions {
    fn default() -> Self {
        Self {
            read_project_detail: true,
            read_blueprint: true,
            read_outlines: true,
            read_character_profiles: true,
            read_foreshadowing: true,
            automatic_writeback: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationPatch {
    pub patch_id: String,
    pub issue_id: Option<String>,
    pub operation: String,
    pub old_text: String,
    pub new_text: String,
    pub reason: String,
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OptimizationSessionSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub name: Option<String>,
    pub status: String,
    pub issues: Option<Value>,
    pub agent_permissions: Option<Value>,
    pub agent_instruction: Option<String>,
    pub fixes_applied: i32,
    pub summary: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OptimizationTurnEventSummary {
    pub id: i64,
    pub turn_id: i64,
    pub seq: i32,
    pub kind: String,
    pub status: String,
    pub public_text: String,
    pub payload: Option<Value>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OptimizationTurnSummary {
    pub id: i64,
    pub session_id: i64,
    pub user_text: String,
    pub status: String,
    pub permissions_snapshot: Option<Value>,
    pub final_reply: Option<String>,
    pub error: Option<String>,
    pub events: Vec<OptimizationTurnEventSummary>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OptimizationPatchSetSummary {
    pub id: i64,
    pub turn_id: i64,
    pub session_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub status: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AnalyzeChapterInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub name: String,
    pub mode: String,
    pub scope: Option<String>,
    pub permissions: Option<OptimizationAgentPermissions>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CreateOptimizationChatSessionInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub name: String,
    pub permissions: Option<OptimizationAgentPermissions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PrepareOptimizationAnalysisSessionInput {
    pub session_id: i64,
    pub mode: String,
    pub scope: Option<String>,
    pub permissions: Option<OptimizationAgentPermissions>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ProposeOptimizationPatchesInput {
    pub session_id: i64,
    pub issue_ids: Option<Vec<String>>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ProposeOptimizationPatchesResponse {
    pub session_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ApplyOptimizationPatchesInput {
    pub session_id: i64,
    pub base_content_hash: String,
    pub patches: Vec<OptimizationPatch>,
    pub confirmed_apply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ApplyOptimizationPatchesResponse {
    pub session_id: i64,
    pub chapter_id: i64,
    pub applied_count: usize,
    pub word_count: i32,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SendOptimizationMessageInput {
    #[schemars(range(min = 1))]
    pub session_id: i64,
    #[schemars(length(min = 1, max = 32768))]
    pub message: String,
    pub permissions: Option<OptimizationAgentPermissions>,
}

/// Start one Optimize chat turn through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartOptimizeChatGenerationInput {
    pub meta: GenerationStartRequestMeta,
    pub request: SendOptimizationMessageInput,
}

/// Start one prepared Optimize analysis through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartOptimizeAnalysisGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub session_id: i64,
}

/// Start one Optimize patch proposal through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartOptimizePatchGenerationInput {
    pub meta: GenerationStartRequestMeta,
    pub request: ProposeOptimizationPatchesInput,
}
