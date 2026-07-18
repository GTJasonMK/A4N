use a4n_core::models::media::media_provenance;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use crate::asset_lineage_sync::{AssetLineageBackfillSummary, AssetLineageSyncError};

use super::super::{changes::set_if_changed, types::MediaUpsertInput};

pub(super) async fn insert_media_provenance<C>(
    db: &C,
    media_asset_id: i64,
    input: &MediaUpsertInput,
    now: NaiveDateTime,
) -> Result<(), AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    media_provenance::ActiveModel {
        media_asset_id: Set(media_asset_id),
        generation_spec_id: Set(input.generation_spec_id),
        prompt_artifact_id: Set(input.prompt_artifact_id),
        provider_job_id: Set(input.provider_job_id.clone()),
        source_media_asset_id: Set(input.source_media_asset_id),
        source_external_path: Set(input.source_external_path.clone()),
        imported_from_user: Set(input.imported_from_user),
        request_snapshot_json: Set(Some(input.request_snapshot_json.clone())),
        created_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(AssetLineageSyncError::Database)?;
    Ok(())
}

pub(super) async fn update_media_provenance_if_needed<C>(
    db: &C,
    existing: media_provenance::Model,
    input: &MediaUpsertInput,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let mut active: media_provenance::ActiveModel = existing.clone().into();
    let mut changed = false;
    set_if_changed(
        &mut active.generation_spec_id,
        existing.generation_spec_id,
        input.generation_spec_id,
        &mut changed,
    );
    set_if_changed(
        &mut active.prompt_artifact_id,
        existing.prompt_artifact_id,
        input.prompt_artifact_id,
        &mut changed,
    );
    set_if_changed(
        &mut active.provider_job_id,
        existing.provider_job_id,
        input.provider_job_id.clone(),
        &mut changed,
    );
    set_if_changed(
        &mut active.source_media_asset_id,
        existing.source_media_asset_id,
        input.source_media_asset_id,
        &mut changed,
    );
    set_if_changed(
        &mut active.source_external_path,
        existing.source_external_path,
        input.source_external_path.clone(),
        &mut changed,
    );
    set_if_changed(
        &mut active.imported_from_user,
        existing.imported_from_user,
        input.imported_from_user,
        &mut changed,
    );
    set_if_changed(
        &mut active.request_snapshot_json,
        existing.request_snapshot_json,
        Some(input.request_snapshot_json.clone()),
        &mut changed,
    );
    if changed {
        active
            .update(db)
            .await
            .map_err(AssetLineageSyncError::Database)?;
        summary.updated_media_provenance += 1;
    }
    Ok(())
}
