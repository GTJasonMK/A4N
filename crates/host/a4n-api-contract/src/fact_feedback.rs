use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FactFeedbackMode {
    Consolidate,
    Rebuild,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunProjectFactFeedbackInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub mode: FactFeedbackMode,
    #[schemars(range(min = 1))]
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MaterialFactChangeSummary {
    pub id: i64,
    pub material_kind: String,
    pub target_id: Option<i64>,
    pub field_path: String,
    pub before: Value,
    pub after: Value,
    pub evidence: Value,
    pub reason: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectFactFeedbackSummary {
    pub project_id: i64,
    pub interval_chapters: i32,
    pub pending_chapters: Vec<i32>,
    pub consolidated_chapters: Vec<i32>,
    pub legacy_chapters: Vec<i32>,
    pub last_consolidated_at: Option<String>,
    pub workflow_run_id: Option<i64>,
    pub workflow_status: Option<String>,
    pub workflow_kind: Option<String>,
    pub can_resume: bool,
    pub error: Option<String>,
    pub changes: Vec<MaterialFactChangeSummary>,
}
