use std::collections::HashMap;

use a4n_core::models::media::media_provenance;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::project::duplicate::{
    ProjectDuplicateError,
    shared::{remap_derived_optional_id, remap_derived_required_id},
};

pub(in crate::project::duplicate::derived) async fn copy_derived_media_provenance<C>(
    db: &C,
    asset_id_map: &HashMap<i64, i64>,
    spec_id_map: &HashMap<i64, i64>,
    prompt_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if asset_id_map.is_empty() {
        return Ok(());
    }
    let old_asset_ids = asset_id_map.keys().copied().collect::<Vec<_>>();
    let records = media_provenance::Entity::find()
        .filter(media_provenance::Column::MediaAssetId.is_in(old_asset_ids))
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let inserts = records
        .into_iter()
        .map(|item| {
            let media_asset_id = remap_derived_required_id(
                asset_id_map,
                item.media_asset_id,
                "media_provenance.media_asset_id",
            )?;
            Ok(media_provenance::ActiveModel {
                media_asset_id: Set(media_asset_id),
                generation_spec_id: Set(remap_derived_optional_id(
                    spec_id_map,
                    item.generation_spec_id,
                    "media_provenance.generation_spec_id",
                )?),
                prompt_artifact_id: Set(remap_derived_optional_id(
                    prompt_id_map,
                    item.prompt_artifact_id,
                    "media_provenance.prompt_artifact_id",
                )?),
                provider_job_id: Set(item.provider_job_id),
                source_media_asset_id: Set(remap_derived_optional_id(
                    asset_id_map,
                    item.source_media_asset_id,
                    "media_provenance.source_media_asset_id",
                )?),
                source_external_path: Set(item.source_external_path),
                imported_from_user: Set(item.imported_from_user),
                request_snapshot_json: Set(item.request_snapshot_json),
                created_at: Set(now),
            })
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    if !inserts.is_empty() {
        media_provenance::Entity::insert_many(inserts)
            .exec_without_returning(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}
