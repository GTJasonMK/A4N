use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_core::models::{
    generation::generation_spec_media_ref,
    media::{media_asset, media_binding, media_provenance},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::BTreeSet;

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

    let candidates = candidate_media_asset_ids
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let still_bound = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::MediaAssetId.is_in(candidates))
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let delete_media_asset_ids = candidate_media_asset_ids
        .difference(&still_bound)
        .copied()
        .collect::<Vec<_>>();
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    let delete_media_asset_ids = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()))
        .exec(db)
        .await?;
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()),
        )
        .exec(db)
        .await?;
    media_asset::Entity::delete_many()
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .exec(db)
        .await?;
    Ok(())
}
