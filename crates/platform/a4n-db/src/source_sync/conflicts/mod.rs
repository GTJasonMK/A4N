mod auto_apply;
mod detach;
mod records;
mod types;

pub(in crate::source_sync) use auto_apply::apply_unconflicted_source_sync_records;
pub use detach::detach_derived_project_record;
pub(in crate::source_sync) use records::{
    insert_source_sync_conflict_records, insert_source_sync_decision_record,
    resolve_source_sync_conflict_record,
};
pub use records::{
    source_sync_conflict_for_decision, source_sync_conflict_records_for_decisions,
    source_sync_conflicts_for_decisions,
};
pub use types::{SourceSyncConflictRecordInput, SourceSyncDecisionRecordInput};

#[cfg(test)]
mod tests;
