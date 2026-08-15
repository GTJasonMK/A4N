use a4n_core::models::{
    generation::{generation_spec, generation_spec_text_anchor},
    project::text_anchor,
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::contracts::SourceSyncAnchorConflictCandidateRecord;
use super::local_state::local_state_for_generation_spec;
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
    let mut records = Vec::new();
    for spec in specs {
        let refs = generation_spec_text_anchor::Entity::find()
            .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(spec.id))
            .all(db)
            .await?;
        if refs.is_empty() {
            continue;
        }
        let local_state = local_state_for_generation_spec(db, &spec).await?;
        for item in refs {
            let old_anchor = text_anchor::Entity::find_by_id(item.text_anchor_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    SourceSyncDbError::NotFound(format!("文本锚点不存在: {}", item.text_anchor_id))
                })?;
            if old_anchor.source_snapshot_id != Some(run.from_snapshot_id) {
                continue;
            }
            let candidates = text_anchor::Entity::find()
                .filter(text_anchor::Column::ProjectId.eq(run.root_project_id))
                .filter(text_anchor::Column::SourceSnapshotId.eq(run.to_snapshot_id))
                .filter(text_anchor::Column::QuoteHash.eq(&old_anchor.quote_hash))
                .all(db)
                .await?;
            records.push(SourceSyncAnchorConflictCandidateRecord {
                spec: spec.clone(),
                local_state,
                text_anchor_role: item.role,
                old_anchor,
                candidates,
            });
        }
    }
    Ok(records)
}
