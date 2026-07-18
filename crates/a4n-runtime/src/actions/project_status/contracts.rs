pub use a4n_orchestrator::project::ProjectRollbackImpactSummary as RollbackImpactSummary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusHistorySummary {
    pub id: i64,
    pub project_id: i64,
    pub from_status: String,
    pub to_status: String,
    pub reason: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPreviewSummary {
    pub from_status: String,
    pub target_status: String,
    pub path: Vec<String>,
    pub affected_chapters: i32,
    pub affected_outlines: i32,
    pub can_rollback: bool,
    pub warning_message: Option<String>,
    pub summary: String,
    pub impacts: Vec<RollbackImpactSummary>,
}
