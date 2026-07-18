use std::collections::HashMap;

use a4n_core::models::media::media_binding;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::project::duplicate::{ProjectDuplicateError, shared::remap_derived_required_id};

use super::rules::remap_derived_media_binding_target;

pub(in crate::project::duplicate) async fn copy_derived_media_bindings<C>(
    db: &C,
    source_project_id: i64,
    asset_id_map: &HashMap<i64, i64>,
    segment_id_map: &HashMap<i64, i64>,
    timeline_id_map: &HashMap<i64, i64>,
    video_id_map: Option<&HashMap<i64, i64>>,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if asset_id_map.is_empty() {
        return Ok(());
    }
    let bindings = media_binding::Entity::find()
        .filter(
            media_binding::Column::MediaAssetId
                .is_in(asset_id_map.keys().copied().collect::<Vec<_>>()),
        )
        .order_by_asc(media_binding::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    for binding in bindings {
        let target_id = remap_derived_media_binding_target(
            source_project_id,
            &binding.target_kind,
            binding.target_id,
            segment_id_map,
            timeline_id_map,
            video_id_map,
        )?;
        media_binding::ActiveModel {
            media_asset_id: Set(remap_derived_required_id(
                asset_id_map,
                binding.media_asset_id,
                "media_binding.media_asset_id",
            )?),
            target_kind: Set(binding.target_kind),
            target_id: Set(target_id),
            role: Set(binding.role),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}
