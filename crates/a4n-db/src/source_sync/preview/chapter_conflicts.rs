use super::{
    persistence::insert_conflict,
    rules::{local_state_for_spec, severity_for_impacted_local_state},
    types::InsertConflictInput,
};
use crate::source_sync::{
    SourceSnapshotChapterChangeRecord, SourceSnapshotProfileChangeRecord, SourceSyncDbError,
    galgame_scene_conflict_candidates_for_chapter_change, generation_specs_for_profile_ref,
    generation_specs_for_source_chapter,
};
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncLocalStateKind,
};
use sea_orm::ConnectionTrait;
use serde_json::json;

pub(super) async fn insert_conflicts_for_change<C>(
    db: &C,
    run: &source_sync_run::Model,
    change: &SourceSnapshotChapterChangeRecord,
) -> Result<Vec<source_sync_conflict::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let specs =
        generation_specs_for_source_chapter(db, run.derived_project_id, change.source_chapter_id)
            .await?;
    if specs.is_empty() {
        return Ok(vec![
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "text_anchor",
                    target_kind: "chapter",
                    target_id: change.source_chapter_id,
                    source_change_kind: change.change_kind,
                    local_state_kind: SourceSyncLocalStateKind::Clean,
                    severity: if change.change_kind == SourceSyncChangeKind::Deleted {
                        SourceSyncConflictSeverity::Blocking
                    } else {
                        SourceSyncConflictSeverity::Info
                    },
                    source_summary_json: change.source_summary_json.clone(),
                    local_summary_json: json!({ "local": "no_generation_spec" }),
                    affected_refs_json: json!({ "source_chapter_id": change.source_chapter_id }),
                },
            )
            .await?,
        ]);
    }
    let mut conflicts = Vec::new();
    for spec in specs {
        let state = local_state_for_spec(db, &spec).await?;
        let severity = severity_for_impacted_local_state(state, change.change_kind);
        conflicts.push(
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "generation_spec",
                    target_kind: "segment",
                    target_id: spec.id,
                    source_change_kind: change.change_kind,
                    local_state_kind: state,
                    severity,
                    source_summary_json: change.source_summary_json.clone(),
                    local_summary_json: json!({
                        "generation_spec_id": spec.id,
                        "target_kind": spec.target_kind,
                        "status": spec.status,
                    }),
                    affected_refs_json: json!({
                        "source_chapter_id": change.source_chapter_id,
                        "source_snapshot_chapter_id": change.source_snapshot_chapter_id,
                    }),
                },
            )
            .await?,
        );
    }
    Ok(conflicts)
}

pub(super) async fn insert_galgame_conflicts_for_change<C>(
    db: &C,
    run: &source_sync_run::Model,
    change: &SourceSnapshotChapterChangeRecord,
) -> Result<Vec<source_sync_conflict::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let candidates = galgame_scene_conflict_candidates_for_chapter_change(db, run, change).await?;
    let mut conflicts = Vec::new();
    for candidate in candidates {
        let scene = candidate.scene;
        conflicts.push(
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "text_anchor",
                    target_kind: "galgame_scene",
                    target_id: scene.id,
                    source_change_kind: change.change_kind,
                    local_state_kind: SourceSyncLocalStateKind::Generated,
                    severity: severity_for_impacted_local_state(
                        SourceSyncLocalStateKind::Generated,
                        change.change_kind,
                    ),
                    source_summary_json: change.source_summary_json.clone(),
                    local_summary_json: json!({
                        "galgame_scene_id": scene.id,
                        "scene_index": scene.scene_index,
                        "title": scene.title,
                        "line_count": candidate.line_count,
                        "anchored_line_count": candidate.anchored_line_count,
                    }),
                    affected_refs_json: json!({
                        "source_chapter_id": change.source_chapter_id,
                        "old_source_snapshot_chapter_id": candidate.old_source_snapshot_chapter_id,
                        "source_snapshot_chapter_id": change.source_snapshot_chapter_id,
                        "galgame_scene_id": scene.id,
                    }),
                },
            )
            .await?,
        );
    }
    Ok(conflicts)
}

pub(super) async fn insert_conflicts_for_profile_change<C>(
    db: &C,
    run: &source_sync_run::Model,
    change: &SourceSnapshotProfileChangeRecord,
) -> Result<Vec<source_sync_conflict::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    if change.change_kind == SourceSyncChangeKind::Added {
        return Ok(Vec::new());
    }
    let refs =
        generation_specs_for_profile_ref(db, run, &change.profile_kind, change.source_profile_id)
            .await?;
    let mut conflicts = Vec::new();
    for item in refs {
        let spec = item.spec;
        let state = local_state_for_spec(db, &spec).await?;
        conflicts.push(
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "profile",
                    target_kind: &change.profile_kind,
                    target_id: change.source_profile_id,
                    source_change_kind: change.change_kind,
                    local_state_kind: state,
                    severity: severity_for_impacted_local_state(state, change.change_kind),
                    source_summary_json: change.source_summary_json.clone(),
                    local_summary_json: json!({
                        "generation_spec_id": spec.id,
                        "entity_role": item.entity_role,
                        "target_kind": spec.target_kind,
                        "status": spec.status,
                    }),
                    affected_refs_json: json!({
                        "profile_kind": change.profile_kind,
                        "source_profile_id": change.source_profile_id,
                        "generation_spec_id": spec.id,
                    }),
                },
            )
            .await?,
        );
    }
    Ok(conflicts)
}
