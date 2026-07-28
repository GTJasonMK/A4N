use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceSyncDecisionInput {
    pub conflict_id: i64,
    pub strategy: String,
    pub decision_payload_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApplySourceSyncInput {
    pub sync_run_id: i64,
    pub decisions: Vec<SourceSyncDecisionInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceSyncRunSummary {
    pub id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub mode: String,
    pub status: String,
    pub summary_json: Value,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceSyncConflictSummary {
    pub id: i64,
    pub sync_run_id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub conflict_scope: String,
    pub target_kind: String,
    pub target_id: i64,
    pub source_change_kind: String,
    pub local_state_kind: String,
    pub severity: String,
    pub status: String,
    pub resolution_strategy: Option<String>,
    pub source_summary_json: Value,
    pub local_summary_json: Value,
    pub affected_refs_json: Value,
    pub decision_json: Option<Value>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceSyncPlanSummary {
    pub run: SourceSyncRunSummary,
    pub conflicts: Vec<SourceSyncConflictSummary>,
}
