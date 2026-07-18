use a4n_core::models::source_sync::source_sync_conflict;
pub use a4n_orchestrator::source_sync::SourceSyncDecisionInput;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplySourceSyncInput {
    pub sync_run_id: i64,
    pub decisions: Vec<SourceSyncDecisionInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSyncRunSummary {
    pub id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub mode: String,
    pub status: String,
    pub summary_json: serde_json::Value,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSyncPlanSummary {
    pub run: SourceSyncRunSummary,
    pub conflicts: Vec<source_sync_conflict::Model>,
}
