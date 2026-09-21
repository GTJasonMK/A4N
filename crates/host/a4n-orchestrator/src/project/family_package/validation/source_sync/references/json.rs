use super::{
    super::contracts::SourceSyncReferenceSets,
    affected::validate_source_sync_affected_refs,
    summaries::{
        validate_source_sync_local_summary_refs, validate_source_sync_source_summary_refs,
    },
};
use a4n_db_models::models::source_sync::source_sync_conflict;

pub(in super::super) fn validate_source_sync_conflict_json_refs(
    conflict: &source_sync_conflict::Model,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let source_summary = conflict
        .source_summary_json
        .as_object()
        .expect("source_summary_json object shape checked before ref validation");
    let local_summary = conflict
        .local_summary_json
        .as_object()
        .expect("local_summary_json object shape checked before ref validation");
    let affected_refs = conflict
        .affected_refs_json
        .as_object()
        .expect("affected_refs_json object shape checked before ref validation");

    validate_source_sync_source_summary_refs(
        &conflict.conflict_scope,
        source_summary,
        conflict.id,
        refs,
    )?;
    validate_source_sync_local_summary_refs(local_summary, conflict.id, refs)?;
    validate_source_sync_affected_refs(affected_refs, conflict.id, refs)
}
