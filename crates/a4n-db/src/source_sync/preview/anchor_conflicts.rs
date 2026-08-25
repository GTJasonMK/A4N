use super::{
    persistence::insert_conflicts,
    rules::{anchor_change_conflict, classify_anchor_change},
    types::InsertConflictInput,
};
use crate::source_sync::{
    SourceSyncAnchorConflictCandidateRecord, SourceSyncDbError,
    SourceSyncGalgameAnchorConflictCandidateRecord, anchor_conflict_candidates_for_run,
    galgame_anchor_conflict_candidates_for_run,
};
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::source_sync::SourceSyncLocalStateKind;
use sea_orm::ConnectionTrait;
use serde_json::json;

pub(super) async fn insert_galgame_anchor_conflicts_for_run<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<Vec<source_sync_conflict::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let candidates = galgame_anchor_conflict_candidates_for_run(db, run).await?;
    let mut inputs = Vec::new();
    for candidate in candidates {
        let SourceSyncGalgameAnchorConflictCandidateRecord {
            line,
            scene_title,
            old_anchor,
            candidates,
        } = candidate;
        let anchor_change = classify_anchor_change(&old_anchor, &candidates);
        let Some((change_kind, new_anchor_id, severity)) =
            anchor_change_conflict(&anchor_change, SourceSyncLocalStateKind::Generated)
        else {
            continue;
        };
        inputs.push(InsertConflictInput {
            conflict_scope: "text_anchor",
            target_kind: "galgame_line",
            target_id: line.id,
            source_change_kind: change_kind,
            local_state_kind: SourceSyncLocalStateKind::Generated,
            severity,
            source_summary_json: json!({
                "old_text_anchor_id": old_anchor.id,
                "new_text_anchor_id": new_anchor_id,
                "quote": old_anchor.quote,
                "quote_hash": old_anchor.quote_hash,
            }),
            local_summary_json: json!({
                "galgame_line_id": line.id,
                "galgame_scene_id": line.scene_id,
                "scene_title": scene_title,
                "line_index": line.line_index,
                "line_type": line.line_type,
                "text": line.text,
            }),
            affected_refs_json: json!({
                "old_text_anchor_id": old_anchor.id,
                "new_text_anchor_id": new_anchor_id,
                "candidate_count": candidates.len(),
                "galgame_line_id": line.id,
                "galgame_scene_id": line.scene_id,
            }),
        });
    }
    insert_conflicts(db, run, inputs).await
}

pub(super) async fn insert_anchor_conflicts_for_run<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<Vec<source_sync_conflict::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let candidates = anchor_conflict_candidates_for_run(db, run).await?;
    let mut inputs = Vec::new();
    for candidate in candidates {
        let SourceSyncAnchorConflictCandidateRecord {
            spec,
            local_state,
            text_anchor_role,
            old_anchor,
            candidates,
        } = candidate;
        let anchor_change = classify_anchor_change(&old_anchor, &candidates);
        let Some((change_kind, new_anchor_id, severity)) =
            anchor_change_conflict(&anchor_change, local_state)
        else {
            continue;
        };
        inputs.push(InsertConflictInput {
            conflict_scope: "text_anchor",
            target_kind: "segment",
            target_id: spec.id,
            source_change_kind: change_kind,
            local_state_kind: local_state,
            severity,
            source_summary_json: json!({
                "old_text_anchor_id": old_anchor.id,
                "new_text_anchor_id": new_anchor_id,
                "quote": old_anchor.quote,
                "quote_hash": old_anchor.quote_hash,
            }),
            local_summary_json: json!({
                "generation_spec_id": spec.id,
                "text_anchor_role": text_anchor_role,
                "target_kind": spec.target_kind,
                "status": spec.status,
            }),
            affected_refs_json: json!({
                "old_text_anchor_id": old_anchor.id,
                "new_text_anchor_id": new_anchor_id,
                "candidate_count": candidates.len(),
            }),
        });
    }
    insert_conflicts(db, run, inputs).await
}
