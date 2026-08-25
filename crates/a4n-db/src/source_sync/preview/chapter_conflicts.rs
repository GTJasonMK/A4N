use super::{
    persistence::insert_conflicts, rules::severity_for_impacted_local_state,
    types::InsertConflictInput,
};
use crate::source_sync::{
    SourceSnapshotChapterChangeRecord, SourceSnapshotProfileChangeRecord, SourceSyncDbError,
    galgame_scene_conflict_candidates_for_chapter_changes, generation_specs_for_profile_refs,
    generation_specs_for_source_chapters, local_states_for_generation_specs,
};
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncLocalStateKind,
};
use sea_orm::ConnectionTrait;
use serde_json::json;
use std::collections::HashMap;

use a4n_core::models::generation::generation_spec;

async fn insert_grouped_conflicts<'a, C>(
    db: &C,
    run: &source_sync_run::Model,
    grouped_inputs: Vec<Vec<InsertConflictInput<'a>>>,
) -> Result<Vec<Vec<source_sync_conflict::Model>>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let group_sizes = grouped_inputs.iter().map(Vec::len).collect::<Vec<_>>();
    let inserted =
        insert_conflicts(db, run, grouped_inputs.into_iter().flatten().collect()).await?;
    split_grouped_conflicts(&inserted, &group_sizes)
}

fn split_grouped_conflicts<T: Clone>(
    inserted: &[T],
    group_sizes: &[usize],
) -> Result<Vec<Vec<T>>, SourceSyncDbError> {
    let expected = group_sizes.iter().sum::<usize>();
    if inserted.len() != expected {
        return Err(SourceSyncDbError::Internal(format!(
            "源同步冲突分组写入返回 {} 行，预期 {} 行",
            inserted.len(),
            expected
        )));
    }
    let mut offset = 0;
    group_sizes
        .iter()
        .map(|size| {
            let end = offset + size;
            let group = inserted[offset..end].to_vec();
            offset = end;
            Ok(group)
        })
        .collect()
}

pub(super) async fn insert_conflicts_for_changes<C>(
    db: &C,
    run: &source_sync_run::Model,
    changes: &[SourceSnapshotChapterChangeRecord],
) -> Result<Vec<Vec<source_sync_conflict::Model>>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    if changes.is_empty() {
        return Ok(Vec::new());
    }
    let source_chapter_ids = changes
        .iter()
        .map(|change| change.source_chapter_id)
        .collect::<Vec<_>>();
    let specs_by_chapter =
        generation_specs_for_source_chapters(db, run.derived_project_id, &source_chapter_ids)
            .await?;
    let specs = changes
        .iter()
        .flat_map(|change| {
            specs_by_chapter
                .get(&change.source_chapter_id)
                .into_iter()
                .flatten()
                .cloned()
        })
        .collect::<Vec<_>>();
    let local_states = local_states_for_generation_specs(db, &specs).await?;
    let mut grouped_inputs = Vec::with_capacity(changes.len());
    for change in changes {
        let chapter_specs = specs_by_chapter
            .get(&change.source_chapter_id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        grouped_inputs.push(conflict_inputs_for_change_with_data(
            change,
            chapter_specs,
            &local_states,
        )?);
    }
    insert_grouped_conflicts(db, run, grouped_inputs).await
}

fn conflict_inputs_for_change_with_data<'a>(
    change: &'a SourceSnapshotChapterChangeRecord,
    specs: &[generation_spec::Model],
    local_states: &HashMap<i64, SourceSyncLocalStateKind>,
) -> Result<Vec<InsertConflictInput<'a>>, SourceSyncDbError> {
    if specs.is_empty() {
        return Ok(vec![InsertConflictInput {
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
        }]);
    }
    let mut inputs = Vec::with_capacity(specs.len());
    for spec in specs {
        let state = local_states.get(&spec.id).cloned().ok_or_else(|| {
            SourceSyncDbError::Internal(format!("未生成 generation_spec {} 的本地状态", spec.id))
        })?;
        let severity = severity_for_impacted_local_state(state, change.change_kind);
        inputs.push(InsertConflictInput {
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
        });
    }
    Ok(inputs)
}

pub(super) async fn insert_galgame_conflicts_for_changes<C>(
    db: &C,
    run: &source_sync_run::Model,
    changes: &[SourceSnapshotChapterChangeRecord],
) -> Result<Vec<Vec<source_sync_conflict::Model>>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let candidates_by_chapter =
        galgame_scene_conflict_candidates_for_chapter_changes(db, run, changes).await?;
    let mut grouped_inputs = Vec::with_capacity(changes.len());
    for change in changes {
        let candidates = candidates_by_chapter
            .get(&change.source_chapter_id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let mut inputs = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let scene = &candidate.scene;
            inputs.push(InsertConflictInput {
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
            });
        }
        grouped_inputs.push(inputs);
    }
    insert_grouped_conflicts(db, run, grouped_inputs).await
}

pub(super) async fn insert_conflicts_for_profile_changes<C>(
    db: &C,
    run: &source_sync_run::Model,
    changes: &[SourceSnapshotProfileChangeRecord],
) -> Result<Vec<Vec<source_sync_conflict::Model>>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let profile_refs = changes
        .iter()
        .filter(|change| change.change_kind != SourceSyncChangeKind::Added)
        .map(|change| (change.profile_kind.clone(), change.source_profile_id))
        .collect::<Vec<_>>();
    if profile_refs.is_empty() {
        return Ok(vec![Vec::new(); changes.len()]);
    }
    let refs_by_profile = generation_specs_for_profile_refs(db, run, &profile_refs).await?;
    let specs = changes
        .iter()
        .filter(|change| change.change_kind != SourceSyncChangeKind::Added)
        .flat_map(|change| {
            refs_by_profile
                .get(&(
                    change.profile_kind.trim().to_string(),
                    change.source_profile_id,
                ))
                .into_iter()
                .flatten()
                .map(|item| item.spec.clone())
        })
        .collect::<Vec<_>>();
    let local_states = local_states_for_generation_specs(db, &specs).await?;
    let mut grouped_inputs = Vec::with_capacity(changes.len());
    for change in changes {
        if change.change_kind == SourceSyncChangeKind::Added {
            grouped_inputs.push(Vec::new());
            continue;
        }
        let refs = refs_by_profile
            .get(&(
                change.profile_kind.trim().to_string(),
                change.source_profile_id,
            ))
            .map(Vec::as_slice)
            .unwrap_or_default();
        grouped_inputs.push(conflict_inputs_for_profile_change_with_data(
            change,
            refs,
            &local_states,
        )?);
    }
    insert_grouped_conflicts(db, run, grouped_inputs).await
}

fn conflict_inputs_for_profile_change_with_data<'a>(
    change: &'a SourceSnapshotProfileChangeRecord,
    refs: &[crate::source_sync::SourceSyncProfileGenerationSpecRefRecord],
    local_states: &HashMap<i64, SourceSyncLocalStateKind>,
) -> Result<Vec<InsertConflictInput<'a>>, SourceSyncDbError> {
    let mut inputs = Vec::with_capacity(refs.len());
    for item in refs {
        let spec = &item.spec;
        let state = local_states.get(&spec.id).cloned().ok_or_else(|| {
            SourceSyncDbError::Internal(format!("未生成 generation_spec {} 的本地状态", spec.id))
        })?;
        inputs.push(InsertConflictInput {
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
        });
    }
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use super::split_grouped_conflicts;

    #[test]
    fn split_grouped_conflicts_preserves_empty_groups_and_order() {
        assert_eq!(
            split_grouped_conflicts(&[1, 2, 3], &[2, 0, 1]).unwrap(),
            vec![vec![1, 2], Vec::<i32>::new(), vec![3]]
        );
    }
}
