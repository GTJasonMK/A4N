use super::super::import_project::VideoStudioProjectImportMediaRowsContext;
use super::super::remap::{
    remap_video_studio_canvas_metadata, remap_video_studio_required_id,
    remap_video_studio_source_external_path,
};
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps,
    VideoStudioProjectImportMediaFileWriter,
};
use a4n_db_models::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_db_models::models::media::{media_asset, media_provenance};
use a4n_domain::CanvasNodeRefKind;
use sea_orm::{ConnectionTrait, EntityTrait, Set};

pub(in crate::canvas::transfer) async fn import_video_studio_media_assets<C, Writer>(
    context: &mut VideoStudioProjectImportMediaRowsContext<'_, C, Writer>,
    assets: &[media_asset::Model],
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    let mut rows = Vec::with_capacity(assets.len());
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
                CanvasNodeRefKind::MediaAsset.as_str(),
                asset.id,
                FILE_PATH_FIELD,
                folder,
                "imported_package_media",
            )
            .map_err(VideoStudioProjectImportError::MediaWrite)?;
        let thumbnail_path = context
            .media_file_writer
            .write_imported_media_file(
                context.project_id,
                CanvasNodeRefKind::MediaAsset.as_str(),
                asset.id,
                THUMBNAIL_PATH_FIELD,
                folder,
                "imported_package_media_thumbnail",
            )
            .map_err(VideoStudioProjectImportError::MediaWrite)?;
        rows.push(media_asset::ActiveModel {
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
        });
    }
    if rows.is_empty() {
        return Ok(());
    }
    let inserted = media_asset::Entity::insert_many(rows)
        .exec_with_returning_many(context.db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if inserted.len() != assets.len() {
        return Err(VideoStudioProjectImportError::InvalidPackage(format!(
            "媒体资产批量写入返回行数不一致: inserted={}, expected={}",
            inserted.len(),
            assets.len()
        )));
    }
    for (asset, row) in assets.iter().zip(inserted) {
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
    let inserts = records
        .iter()
        .map(|record| {
            Ok(media_provenance::ActiveModel {
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
                provider_job_id: Set(None),
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
            })
        })
        .collect::<Result<Vec<_>, VideoStudioProjectImportError>>()?;
    if !inserts.is_empty() {
        media_provenance::Entity::insert_many(inserts)
            .exec_without_returning(db)
            .await
            .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}
