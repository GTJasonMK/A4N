use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncDecisionActor,
    SourceSyncLocalStateKind, SourceSyncResolutionStrategy,
};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SourceSyncDecisionRecordInput {
    pub conflict_id: i64,
    pub decided_by: SourceSyncDecisionActor,
    pub strategy: SourceSyncResolutionStrategy,
    pub decision_payload_json: Value,
}

#[derive(Debug, Clone)]
pub struct SourceSyncConflictRecordInput {
    pub conflict_scope: String,
    pub target_kind: String,
    pub target_id: i64,
    pub source_change_kind: SourceSyncChangeKind,
    pub local_state_kind: SourceSyncLocalStateKind,
    pub severity: SourceSyncConflictSeverity,
    pub source_summary_json: Value,
    pub local_summary_json: Value,
    pub affected_refs_json: Value,
}
