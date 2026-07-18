use super::types::AnchorChange;
use crate::source_sync::{SourceSyncDbError, local_state_for_generation_spec};
use a4n_core::models::{generation::generation_spec, project::text_anchor};
use sea_orm::ConnectionTrait;

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
    local_state: &str,
) -> Option<(&'static str, Option<i64>, &'static str)> {
    match change {
        AnchorChange::Unchanged => None,
        AnchorChange::Moved(new_anchor_id) => Some((
            "moved",
            Some(*new_anchor_id),
            severity_for_impacted_local_state(local_state, "moved"),
        )),
        AnchorChange::Missing => Some(("deleted", None, "blocking")),
        AnchorChange::Ambiguous => Some(("split", None, "blocking")),
    }
}

pub(super) fn severity_for_impacted_local_state(
    local_state: &str,
    source_change_kind: &str,
) -> &'static str {
    if source_change_kind == "deleted" {
        return "blocking";
    }
    match local_state {
        "user_edited" | "generated" | "imported" | "assembled" | "exported" => "blocking",
        "clean" | "draft" => "warning",
        _ => "info",
    }
}

pub(super) async fn local_state_for_spec<C>(
    db: &C,
    spec: &generation_spec::Model,
) -> Result<String, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    local_state_for_generation_spec(db, spec).await
}
