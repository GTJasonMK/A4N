use super::super::import_project::VideoStudioProjectImportMediaRowsContext;
use super::super::remap::{
    remap_video_studio_canvas_metadata, remap_video_studio_required_id,
    remap_video_studio_source_external_path,
};
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps,
    VideoStudioProjectImportMediaFileWriter,
};
use super::super::{MEDIA_FIELD_FILE, MEDIA_FIELD_THUMBNAIL, MEDIA_OWNER_MEDIA_ASSET};
use a4n_core::models::media::{media_asset, media_provenance};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(in crate::canvas::transfer) async fn import_video_studio_media_assets<C, Writer>(
    context: &mut VideoStudioProjectImportMediaRowsContext<'_, C, Writer>,
    assets: &[media_asset::Model],
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    for asset in assets {
        let folder = match asset.media_kind.as_str() {
            "image" => "media/image",
            "video" => "media/video",
            "audio" => "media/audio",
            other => {
                return Err(VideoStudioProjectImportError::InvalidPackage(format!(
                    "媒体资产类型无效: media_asset_id={}, media_kind={other}",
                    asset.id
                )));
            }
        };
        let file_path = context
            .media_file_writer
            .write_imported_media_file(
                context.project_id,
                MEDIA_OWNER_MEDIA_ASSET,
                asset.id,
                MEDIA_FIELD_FILE,
                folder,
                "imported_package_media",
            )
            .map_err(VideoStudioProjectImportError::MediaWrite)?;
        let thumbnail_path = context
            .media_file_writer
            .write_imported_media_file(
                context.project_id,
                MEDIA_OWNER_MEDIA_ASSET,
                asset.id,
                MEDIA_FIELD_THUMBNAIL,
                folder,
                "imported_package_media_thumbnail",
            )
            .map_err(VideoStudioProjectImportError::MediaWrite)?;
        let row = media_asset::ActiveModel {
            project_id: Set(context.project_id),
            root_project_id: Set(context.project_id),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            media_kind: Set(asset.media_kind.clone()),
            origin: Set(asset.origin.clone()),
            file_path: Set(file_path),
            thumbnail_path: Set(thumbnail_path),
            width: Set(asset.width),
            height: Set(asset.height),
            duration_secs: Set(asset.duration_secs),
            fps: Set(asset.fps),
            file_size: Set(asset.file_size),
            checksum: Set(asset.checksum.clone()),
            status: Set(asset.status.clone()),
            error_message: Set(asset.error_message.clone()),
            created_at: Set(context.now),
            updated_at: Set(context.now),
            ..Default::default()
        }
        .insert(context.db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        context.maps.media_asset_ids.insert(asset.id, row.id);
    }
    Ok(())
}

pub(in crate::canvas::transfer) async fn import_video_studio_media_provenance<C>(
    db: &C,
    records: &[media_provenance::Model],
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for record in records {
        media_provenance::ActiveModel {
            media_asset_id: Set(remap_video_studio_required_id(
                &maps.media_asset_ids,
                record.media_asset_id,
                "media_provenance.media_asset_id",
            )?),
            generation_spec_id: Set(record
                .generation_spec_id
                .map(|old_id| {
                    remap_video_studio_required_id(
                        &maps.generation_spec_ids,
                        old_id,
                        "media_provenance.generation_spec_id",
                    )
                })
                .transpose()?),
            prompt_artifact_id: Set(record
                .prompt_artifact_id
                .map(|old_id| {
                    remap_video_studio_required_id(
                        &maps.prompt_artifact_ids,
                        old_id,
                        "media_provenance.prompt_artifact_id",
                    )
                })
                .transpose()?),
            provider_job_id: Set(record.provider_job_id.clone()),
            source_media_asset_id: Set(record
                .source_media_asset_id
                .map(|old_id| {
                    remap_video_studio_required_id(
                        &maps.media_asset_ids,
                        old_id,
                        "media_provenance.source_media_asset_id",
                    )
                })
                .transpose()?),
            source_external_path: Set(remap_video_studio_source_external_path(
                record.source_external_path.as_deref(),
                maps,
            )?),
            imported_from_user: Set(record.imported_from_user),
            request_snapshot_json: Set(record
                .request_snapshot_json
                .clone()
                .map(|value| remap_video_studio_canvas_metadata(value, maps))
                .transpose()?),
            created_at: Set(now),
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}
