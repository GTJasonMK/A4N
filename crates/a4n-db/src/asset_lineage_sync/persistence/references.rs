use std::collections::BTreeMap;

use crate::asset_lineage_sync::{AssetLineageBackfillSummary, AssetLineageSyncError};
use a4n_core::models::{generation::generation_spec_media_ref, media::media_binding};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, Set,
};
use serde_json::Value;

pub(in crate::asset_lineage_sync) async fn sync_video_prompt_media_refs<C>(
    db: &C,
    generation_spec_id: i64,
    reference_image_ids_json: Option<&Value>,
    image_media_by_legacy_id: &BTreeMap<i64, i64>,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let Some(reference_ids) = reference_image_ids_json.and_then(Value::as_array) else {
        return Ok(());
    };
    for image_id in reference_ids.iter().filter_map(Value::as_i64) {
        if let Some(media_asset_id) = image_media_by_legacy_id.get(&image_id).copied() {
            ensure_generation_media_ref(
                db,
                generation_spec_id,
                media_asset_id,
                "reference",
                summary,
            )
            .await?;
        }
    }
    Ok(())
}

async fn ensure_generation_media_ref<C>(
    db: &C,
    generation_spec_id: i64,
    media_asset_id: i64,
    role: &str,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let exists = generation_spec_media_ref::Entity::find()
        .filter(generation_spec_media_ref::Column::GenerationSpecId.eq(generation_spec_id))
        .filter(generation_spec_media_ref::Column::MediaAssetId.eq(media_asset_id))
        .filter(generation_spec_media_ref::Column::Role.eq(role))
        .count(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    if exists > 0 {
        return Ok(());
    }
    generation_spec_media_ref::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        media_asset_id: Set(media_asset_id),
        role: Set(role.to_string()),
    }
    .insert(db)
    .await
    .map_err(AssetLineageSyncError::Database)?;
    summary.created_generation_media_refs += 1;
    Ok(())
}

pub(in crate::asset_lineage_sync) async fn ensure_media_binding<C>(
    db: &C,
    media_asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let exists = media_binding::Entity::find()
        .filter(media_binding::Column::MediaAssetId.eq(media_asset_id))
        .filter(media_binding::Column::TargetKind.eq(target_kind))
        .filter(media_binding::Column::TargetId.eq(target_id))
        .filter(media_binding::Column::Role.eq(role))
        .count(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    if exists > 0 {
        return Ok(());
    }
    media_binding::ActiveModel {
        media_asset_id: Set(media_asset_id),
        target_kind: Set(target_kind.to_string()),
        target_id: Set(target_id),
        role: Set(role.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AssetLineageSyncError::Database)?;
    summary.created_media_bindings += 1;
    Ok(())
}
