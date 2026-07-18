use a4n_orchestrator::optimize::{
    OptimizationPatch, OptimizeAgentPermissions as OptimizationAgentPermissions,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSessionSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub name: Option<String>,
    pub status: String,
    pub issues: Option<serde_json::Value>,
    pub agent_permissions: Option<serde_json::Value>,
    pub agent_instruction: Option<String>,
    pub fixes_applied: i32,
    pub summary: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationTurnEventSummary {
    pub id: i64,
    pub turn_id: i64,
    pub seq: i32,
    pub kind: String,
    pub status: String,
    pub public_text: String,
    pub payload: Option<serde_json::Value>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationTurnSummary {
    pub id: i64,
    pub session_id: i64,
    pub user_text: String,
    pub status: String,
    pub permissions_snapshot: Option<serde_json::Value>,
    pub final_reply: Option<String>,
    pub error: Option<String>,
    pub events: Vec<OptimizationTurnEventSummary>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeChapterInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub name: String,
    pub mode: String,
    pub scope: Option<String>,
    pub permissions: Option<OptimizationAgentPermissions>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOptimizationChatSessionInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub name: String,
    pub permissions: Option<OptimizationAgentPermissions>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrepareOptimizationAnalysisSessionInput {
    pub session_id: i64,
    pub mode: String,
    pub scope: Option<String>,
    pub permissions: Option<OptimizationAgentPermissions>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProposeOptimizationPatchesInput {
    pub session_id: i64,
    pub issue_ids: Option<Vec<String>>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposeOptimizationPatchesResponse {
    pub session_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyOptimizationPatchesInput {
    pub session_id: i64,
    pub base_content_hash: String,
    pub patches: Vec<OptimizationPatch>,
    pub confirmed_apply: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOptimizationPatchesResponse {
    pub session_id: i64,
    pub chapter_id: i64,
    pub applied_count: usize,
    pub word_count: i32,
    pub content_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendOptimizationMessageInput {
    pub session_id: i64,
    pub message: String,
    pub permissions: Option<OptimizationAgentPermissions>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendOptimizationMessageResponse {
    pub session_id: i64,
    pub turn: OptimizationTurnSummary,
}

#[derive(Debug, Clone, Copy)]
pub struct OptimizationTaskScope {
    pub project_id: i64,
    pub total: u32,
}
