use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_core::models::media::media_provenance;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::{BTreeSet, HashSet};

pub async fn media_asset_ids_for_generation_specs<C>(
    db: &C,
    generation_spec_ids: &[i64],
) -> Result<Vec<i64>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(Vec::new());
    }
    for generation_spec_id in generation_spec_ids {
        validate_positive_id(*generation_spec_id, "generation_spec_id")?;
    }

    media_provenance::Entity::find()
        .select_only()
        .column(media_provenance::Column::MediaAssetId)
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(SourceSyncDbError::from)
}

pub async fn delete_unbound_project_media_asset_records<C>(
    db: &C,
    project_id: i64,
    candidate_media_asset_ids: BTreeSet<i64>,
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(project_id, "project_id")?;
    if candidate_media_asset_ids.is_empty() {
        return Ok(());
    }
    for media_asset_id in &candidate_media_asset_ids {
        validate_positive_id(*media_asset_id, "media_asset_id")?;
    }

    crate::media::delete_unbound_project_media_assets(
        db,
        project_id,
        candidate_media_asset_ids
            .into_iter()
            .collect::<HashSet<_>>(),
    )
    .await?;
    Ok(())
}
