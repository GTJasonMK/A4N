use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SourceSyncDecisionRecordInput {
    pub conflict_id: i64,
    pub decided_by: String,
    pub strategy: String,
    pub decision_payload_json: Value,
}

#[derive(Debug, Clone)]
pub struct SourceSyncConflictRecordInput {
    pub conflict_scope: String,
    pub target_kind: String,
    pub target_id: i64,
    pub source_change_kind: String,
    pub local_state_kind: String,
    pub severity: String,
    pub source_summary_json: Value,
    pub local_summary_json: Value,
    pub affected_refs_json: Value,
}
