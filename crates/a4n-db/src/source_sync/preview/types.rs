use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncLocalStateKind,
};
use serde_json::Value;

pub(super) struct InsertConflictInput<'a> {
    pub(super) conflict_scope: &'a str,
    pub(super) target_kind: &'a str,
    pub(super) target_id: i64,
    pub(super) source_change_kind: SourceSyncChangeKind,
    pub(super) local_state_kind: SourceSyncLocalStateKind,
    pub(super) severity: SourceSyncConflictSeverity,
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
