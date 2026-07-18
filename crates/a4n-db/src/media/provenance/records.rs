use a4n_core::models::media::{media_asset, media_provenance};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use crate::media::{CreateMediaAssetRecordInput, MediaDbError, MediaProvenanceRecordInput};

use super::references::validate_media_provenance_refs;

pub(in crate::media) async fn insert_media_asset_with_provenance_record<C>(
    db: &C,
    input: CreateMediaAssetRecordInput,
) -> Result<media_asset::Model, MediaDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let asset = media_asset::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        source_chapter_id: Set(input.source_chapter_id),
        source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
        media_kind: Set(input.media_kind),
        origin: Set(input.origin),
        file_path: Set(input.file_path),
        thumbnail_path: Set(input.thumbnail_path),
        width: Set(input.width),
        height: Set(input.height),
        duration_secs: Set(input.duration_secs),
        fps: Set(input.fps),
        file_size: Set(input.file_size),
        checksum: Set(input.checksum),
        status: Set(input.status),
        error_message: Set(input.error_message),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(MediaDbError::Database)?;

    insert_media_provenance_record(db, &asset, input.provenance, now).await?;
    Ok(asset)
}

async fn insert_media_provenance_record<C>(
    db: &C,
    asset: &media_asset::Model,
    input: MediaProvenanceRecordInput,
    created_at: NaiveDateTime,
) -> Result<media_provenance::Model, MediaDbError>
where
    C: ConnectionTrait,
{
    validate_media_provenance_refs(db, asset, &input).await?;

    media_provenance::ActiveModel {
        media_asset_id: Set(asset.id),
        generation_spec_id: Set(input.generation_spec_id),
        prompt_artifact_id: Set(input.prompt_artifact_id),
        provider_job_id: Set(input.provider_job_id),
        source_media_asset_id: Set(input.source_media_asset_id),
        source_external_path: Set(input.source_external_path),
        imported_from_user: Set(input.imported_from_user),
        request_snapshot_json: Set(input.request_snapshot_json),
        created_at: Set(created_at),
    }
    .insert(db)
    .await
    .map_err(MediaDbError::Database)
}
