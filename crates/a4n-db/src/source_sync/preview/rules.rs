use super::types::AnchorChange;
use a4n_core::models::project::text_anchor;
use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncLocalStateKind,
};

pub(super) fn classify_anchor_change(
    old_anchor: &text_anchor::Model,
    candidates: &[text_anchor::Model],
) -> AnchorChange {
    if candidates.is_empty() {
        return AnchorChange::Missing;
    }
    if candidates.len() > 1 {
        return AnchorChange::Ambiguous;
    }
    let new_anchor = &candidates[0];
    if new_anchor.source_chapter_id == old_anchor.source_chapter_id
        && new_anchor.paragraph_index == old_anchor.paragraph_index
        && new_anchor.start_offset == old_anchor.start_offset
        && new_anchor.end_offset == old_anchor.end_offset
    {
        AnchorChange::Unchanged
    } else {
        AnchorChange::Moved(new_anchor.id)
    }
}

pub(super) fn anchor_change_conflict(
    change: &AnchorChange,
    local_state: SourceSyncLocalStateKind,
) -> Option<(
    SourceSyncChangeKind,
    Option<i64>,
    SourceSyncConflictSeverity,
)> {
    match change {
        AnchorChange::Unchanged => None,
        AnchorChange::Moved(new_anchor_id) => Some((
            SourceSyncChangeKind::Moved,
            Some(*new_anchor_id),
            severity_for_impacted_local_state(local_state, SourceSyncChangeKind::Moved),
        )),
        AnchorChange::Missing => Some((
            SourceSyncChangeKind::Deleted,
            None,
            SourceSyncConflictSeverity::Blocking,
        )),
        AnchorChange::Ambiguous => Some((
            SourceSyncChangeKind::Split,
            None,
            SourceSyncConflictSeverity::Blocking,
        )),
    }
}

pub(super) fn severity_for_impacted_local_state(
    local_state: SourceSyncLocalStateKind,
    source_change_kind: SourceSyncChangeKind,
) -> SourceSyncConflictSeverity {
    if source_change_kind == SourceSyncChangeKind::Deleted {
        return SourceSyncConflictSeverity::Blocking;
    }
    match local_state {
        SourceSyncLocalStateKind::UserEdited
        | SourceSyncLocalStateKind::Generated
        | SourceSyncLocalStateKind::Imported
        | SourceSyncLocalStateKind::Assembled => SourceSyncConflictSeverity::Blocking,
        SourceSyncLocalStateKind::Clean | SourceSyncLocalStateKind::Draft => {
            SourceSyncConflictSeverity::Warning
        }
    }
}
