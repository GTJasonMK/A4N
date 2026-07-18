use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::FreeformCopyMaps;
use crate::project::duplicate::freeform::remap::{
    remap_freeform_metadata, remap_freeform_source_external_path,
};
use crate::project::duplicate::shared::{remap_derived_optional_id, remap_derived_required_id};
use a4n_core::models::media::media_provenance;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;

pub(in crate::project::duplicate::freeform) async fn copy_freeform_media_provenance_record<C>(
    db: &C,
    asset_id_map: &HashMap<i64, i64>,
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if asset_id_map.is_empty() {
        return Ok(());
    }
    let records = media_provenance::Entity::find()
        .filter(media_provenance::Column::MediaAssetId.is_in(asset_id_map.keys().copied()))
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    for item in records {
        let request_snapshot_json = item
            .request_snapshot_json
            .map(|value| remap_freeform_metadata(value, maps))
            .transpose()?;
        media_provenance::ActiveModel {
            media_asset_id: Set(remap_derived_required_id(
                asset_id_map,
                item.media_asset_id,
                "media_provenance.media_asset_id",
            )?),
            generation_spec_id: Set(remap_derived_optional_id(
                &maps.generation_spec_ids,
                item.generation_spec_id,
                "media_provenance.generation_spec_id",
            )?),
            prompt_artifact_id: Set(remap_derived_optional_id(
                &maps.prompt_artifact_ids,
                item.prompt_artifact_id,
                "media_provenance.prompt_artifact_id",
            )?),
            provider_job_id: Set(item.provider_job_id),
            source_media_asset_id: Set(remap_derived_optional_id(
                &maps.media_asset_ids,
                item.source_media_asset_id,
                "media_provenance.source_media_asset_id",
            )?),
            source_external_path: Set(remap_freeform_source_external_path(
                item.source_external_path.as_deref(),
                maps,
            )?),
            imported_from_user: Set(item.imported_from_user),
            request_snapshot_json: Set(request_snapshot_json),
            created_at: Set(now),
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}
