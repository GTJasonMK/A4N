use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncConflictStatus,
    SourceSyncLocalStateKind, SourceSyncResolutionStrategy, SourceSyncRunMode, SourceSyncRunStatus,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSyncProjectInput {
    pub derived_project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceSyncDecisionInput {
    pub conflict_id: i64,
    pub strategy: SourceSyncResolutionStrategy,
    pub decision_payload_json: Map<String, Value>,
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
    pub mode: SourceSyncRunMode,
    pub status: SourceSyncRunStatus,
    pub summary_json: Map<String, Value>,
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
    pub source_change_kind: SourceSyncChangeKind,
    pub local_state_kind: SourceSyncLocalStateKind,
    pub severity: SourceSyncConflictSeverity,
    pub status: SourceSyncConflictStatus,
    pub resolution_strategy: Option<SourceSyncResolutionStrategy>,
    pub source_summary_json: Map<String, Value>,
    pub local_summary_json: Map<String, Value>,
    pub affected_refs_json: Map<String, Value>,
    pub decision_json: Option<Map<String, Value>>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceSyncPlanSummary {
    pub run: SourceSyncRunSummary,
    pub conflicts: Vec<SourceSyncConflictSummary>,
}

/// Host-facing source synchronization read boundary.
#[async_trait::async_trait]
pub trait SourceSyncReadApplicationPort: Send + Sync {
    async fn preview(
        &self,
        user_id: i64,
        input: SourceSyncProjectInput,
    ) -> Result<SourceSyncPlanSummary, a4n_error::AppError>;
}

/// Host-facing source synchronization mutation boundary.
///
/// Conflict persistence stays with the source-sync owner; Runtime only
/// supplies prompt rendering for the Galgame regeneration branch.
#[async_trait::async_trait]
pub trait SourceSyncMutationApplicationPort: Send + Sync {
    async fn apply(
        &self,
        user_id: i64,
        input: ApplySourceSyncInput,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
    ) -> Result<SourceSyncPlanSummary, a4n_error::AppError>;

    async fn detach(
        &self,
        user_id: i64,
        input: SourceSyncProjectInput,
    ) -> Result<(), a4n_error::AppError>;
}
