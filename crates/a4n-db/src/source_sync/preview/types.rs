use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SourceSyncPreviewRecord {
    pub run: source_sync_run::Model,
    pub conflicts: Vec<source_sync_conflict::Model>,
}

pub(super) struct InsertConflictInput<'a> {
    pub(super) conflict_scope: &'a str,
    pub(super) target_kind: &'a str,
    pub(super) target_id: i64,
    pub(super) source_change_kind: &'a str,
    pub(super) local_state_kind: &'a str,
    pub(super) severity: &'a str,
    pub(super) source_summary_json: Value,
    pub(super) local_summary_json: Value,
    pub(super) affected_refs_json: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AnchorChange {
    Unchanged,
    Moved(i64),
    Missing,
    Ambiguous,
}
