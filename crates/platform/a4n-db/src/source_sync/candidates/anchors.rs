use a4n_db_models::models::{
    generation::{generation_spec, generation_spec_text_anchor},
    project::text_anchor,
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashMap;

use super::contracts::SourceSyncAnchorConflictCandidateRecord;
use super::local_state::local_states_for_generation_specs;
use crate::source_sync::{SourceSyncDbError, validate_positive_id};

pub async fn anchor_conflict_candidates_for_run<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<Vec<SourceSyncAnchorConflictCandidateRecord>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.from_snapshot_id, "from_snapshot_id")?;
    validate_positive_id(run.to_snapshot_id, "to_snapshot_id")?;

    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(run.derived_project_id))
        .filter(generation_spec::Column::RootProjectId.eq(run.root_project_id))
        .all(db)
        .await?;
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let spec_ids = specs.iter().map(|spec| spec.id).collect::<Vec<_>>();
    let refs = generation_spec_text_anchor::Entity::find()
        .select_only()
        .column(generation_spec_text_anchor::Column::GenerationSpecId)
        .column(generation_spec_text_anchor::Column::TextAnchorId)
        .column(generation_spec_text_anchor::Column::Role)
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.is_in(spec_ids))
        .into_tuple::<(i64, i64, String)>()
        .all(db)
        .await?;
    if refs.is_empty() {
        return Ok(Vec::new());
    }
    let refs_by_spec = refs.into_iter().fold(
        HashMap::<i64, Vec<(i64, i64, String)>>::new(),
        |mut refs_by_spec, reference| {
            refs_by_spec.entry(reference.0).or_default().push(reference);
            refs_by_spec
        },
    );
    let anchor_ids = refs_by_spec
        .values()
        .flatten()
        .map(|reference| reference.1)
        .collect::<Vec<_>>();
    let anchors_by_id = text_anchor::Entity::find()
        .filter(text_anchor::Column::Id.is_in(anchor_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|anchor| (anchor.id, anchor))
        .collect::<HashMap<_, _>>();
    let source_anchor_ids = refs_by_spec
        .values()
        .flatten()
        .filter_map(|reference| anchors_by_id.get(&reference.1))
        .filter(|anchor| anchor.source_snapshot_id == Some(run.from_snapshot_id))
        .map(|anchor| anchor.quote_hash.clone())
        .collect::<Vec<_>>();
    let specs_with_refs = specs
        .iter()
        .filter(|spec| refs_by_spec.contains_key(&spec.id))
        .cloned()
        .collect::<Vec<_>>();
    let candidates_query = async {
        if source_anchor_ids.is_empty() {
            Ok::<HashMap<String, Vec<text_anchor::Model>>, SourceSyncDbError>(HashMap::new())
        } else {
            Ok(text_anchor::Entity::find()
                .filter(text_anchor::Column::ProjectId.eq(run.root_project_id))
                .filter(text_anchor::Column::SourceSnapshotId.eq(run.to_snapshot_id))
                .filter(text_anchor::Column::QuoteHash.is_in(source_anchor_ids))
                .all(db)
                .await?
                .into_iter()
                .fold(
                    HashMap::<String, Vec<text_anchor::Model>>::new(),
                    |mut candidates_by_quote_hash, candidate| {
                        candidates_by_quote_hash
                            .entry(candidate.quote_hash.clone())
                            .or_default()
                            .push(candidate);
                        candidates_by_quote_hash
                    },
                ))
        }
    };
    let (candidates_result, local_states_result) = futures::join!(
        candidates_query,
        local_states_for_generation_specs(db, &specs_with_refs),
    );
    let candidates_by_quote_hash = candidates_result?;
    let local_states = local_states_result?;
    let mut records = Vec::new();
    for spec in specs {
        let Some(refs) = refs_by_spec.get(&spec.id) else {
            continue;
        };
        let local_state = local_states.get(&spec.id).cloned().ok_or_else(|| {
            SourceSyncDbError::Internal(format!("未生成 generation_spec {} 的本地状态", spec.id))
        })?;
        for item in refs {
            let old_anchor = anchors_by_id.get(&item.1).cloned().ok_or_else(|| {
                SourceSyncDbError::NotFound(format!("文本锚点不存在: {}", item.1))
            })?;
            if old_anchor.source_snapshot_id != Some(run.from_snapshot_id) {
                continue;
            }
            let candidates = candidates_by_quote_hash
                .get(&old_anchor.quote_hash)
                .cloned()
                .unwrap_or_default();
            records.push(SourceSyncAnchorConflictCandidateRecord {
                spec: spec.clone(),
                local_state,
                text_anchor_role: item.2.clone(),
                old_anchor,
                candidates,
            });
        }
    }
    Ok(records)
}
