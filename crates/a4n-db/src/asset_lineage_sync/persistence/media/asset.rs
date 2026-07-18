use a4n_core::models::media::media_asset;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use crate::asset_lineage_sync::{
    AssetLineageBackfillSummary, AssetLineageSyncError, FamilyLineageContext,
};

use super::super::{
    changes::{set_if_changed, set_string_if_changed},
    types::MediaUpsertInput,
};
use super::lookup::find_media_asset_by_key;
use super::provenance::{insert_media_provenance, update_media_provenance_if_needed};

pub(in crate::asset_lineage_sync) async fn upsert_media_asset<C>(
    db: &C,
    family: &FamilyLineageContext,
    input: MediaUpsertInput,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<media_asset::Model, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let existing = find_media_asset_by_key(db, family, &input.canonical_key).await?;
    if let Some((existing_asset, existing_provenance)) = existing {
        let mut active: media_asset::ActiveModel = existing_asset.clone().into();
        let mut changed = false;
        set_if_changed(
            &mut active.project_id,
            existing_asset.project_id,
            input.project_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.root_project_id,
            existing_asset.root_project_id,
            input.root_project_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_snapshot_id,
            existing_asset.source_snapshot_id,
            input.source_snapshot_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_chapter_id,
            existing_asset.source_chapter_id,
            input.source_chapter_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_snapshot_chapter_id,
            existing_asset.source_snapshot_chapter_id,
            input.source_snapshot_chapter_id,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.media_kind,
            &existing_asset.media_kind,
            input.media_kind.clone(),
            &mut changed,
        );
        set_string_if_changed(
            &mut active.origin,
            &existing_asset.origin,
            input.origin.clone(),
            &mut changed,
        );
        set_if_changed(
            &mut active.file_path,
            existing_asset.file_path.clone(),
            input.file_path.clone(),
            &mut changed,
        );
        set_if_changed(
            &mut active.thumbnail_path,
            existing_asset.thumbnail_path.clone(),
            input.thumbnail_path.clone(),
            &mut changed,
        );
        set_if_changed(
            &mut active.width,
            existing_asset.width,
            input.width,
            &mut changed,
        );
        set_if_changed(
            &mut active.height,
            existing_asset.height,
            input.height,
            &mut changed,
        );
        set_if_changed(
            &mut active.duration_secs,
            existing_asset.duration_secs,
            input.duration_secs,
            &mut changed,
        );
        set_if_changed(&mut active.fps, existing_asset.fps, input.fps, &mut changed);
        set_if_changed(
            &mut active.file_size,
            existing_asset.file_size,
            input.file_size,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.status,
            &existing_asset.status,
            input.status.clone(),
            &mut changed,
        );
        set_if_changed(
            &mut active.error_message,
            existing_asset.error_message.clone(),
            input.error_message.clone(),
            &mut changed,
        );
        let asset = if changed {
            active.updated_at = Set(Utc::now().naive_utc());
            summary.updated_media_assets += 1;
            active
                .update(db)
                .await
                .map_err(AssetLineageSyncError::Database)?
        } else {
            existing_asset
        };
        update_media_provenance_if_needed(db, existing_provenance, &input, summary).await?;
        return Ok(asset);
    }

    let now = Utc::now().naive_utc();
    let asset = media_asset::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        source_chapter_id: Set(input.source_chapter_id),
        source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
        media_kind: Set(input.media_kind.clone()),
        origin: Set(input.origin.clone()),
        file_path: Set(input.file_path.clone()),
        thumbnail_path: Set(input.thumbnail_path.clone()),
        width: Set(input.width),
        height: Set(input.height),
        duration_secs: Set(input.duration_secs),
        fps: Set(input.fps),
        file_size: Set(input.file_size),
        checksum: Set(None),
        status: Set(input.status.clone()),
        error_message: Set(input.error_message.clone()),
        created_at: Set(input.created_at),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AssetLineageSyncError::Database)?;
    insert_media_provenance(db, asset.id, &input, now).await?;
    summary.created_media_assets += 1;
    summary.created_media_provenance += 1;
    Ok(asset)
}
