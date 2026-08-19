use super::super::{
    remap::remap_video_studio_optional_id,
    types::{VideoStudioProjectImportError, VideoStudioProjectImportMediaFileWriter},
};
use super::VideoStudioProjectImportMediaRowsContext;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_domain::CanvasNodeRefKind;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use std::collections::HashMap;

pub(in crate::canvas::transfer) async fn import_video_studio_generated_images<C, Writer>(
    context: &mut VideoStudioProjectImportMediaRowsContext<'_, C, Writer>,
    images: &[generated_image::Model],
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    for image in images {
        let target_relative = context
            .media_file_writer
            .write_imported_media_file(
                context.project_id,
                CanvasNodeRefKind::GeneratedImage.as_str(),
                image.id,
                FILE_PATH_FIELD,
                "images",
                "imported_package_image",
            )
            .map_err(VideoStudioProjectImportError::MediaWrite)?
            .ok_or_else(|| {
                VideoStudioProjectImportError::InvalidPackage(format!(
                    "生成图片缺少媒体文件: {}",
                    image.id
                ))
            })?;
        let row = generated_image::ActiveModel {
            project_id: Set(context.project_id),
            image_type: Set(image.image_type.clone()),
            file_path: Set(target_relative.clone()),
            prompt: Set(image.prompt.clone()),
            model: Set(image.model.clone()),
            width: Set(image.width),
            height: Set(image.height),
            file_size: Set(image.file_size),
            ai_score: Set(image.ai_score),
            is_favorite: Set(image.is_favorite),
            chapter_id: Set(None),
            panel_id: Set(image.panel_id.clone()),
            metadata_json: Set(image.metadata_json.clone()),
            created_at: Set(context.now),
            ..Default::default()
        }
        .insert(context.db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        context.maps.image_ids.insert(image.id, row.id);
        context
            .maps
            .image_file_paths
            .insert(image.id, target_relative);
    }
    Ok(())
}

pub(in crate::canvas::transfer) async fn import_video_studio_generated_videos<C, Writer>(
    context: &mut VideoStudioProjectImportMediaRowsContext<'_, C, Writer>,
    videos: &[generated_video::Model],
) -> Result<HashMap<i64, generated_video::Model>, VideoStudioProjectImportError>
where
    C: ConnectionTrait,
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    let mut imported = HashMap::new();
    for video in videos {
        if !generated_video::is_terminal_status(&video.status) {
            return Err(VideoStudioProjectImportError::InvalidPackage(format!(
                "视频画布项目包不能导入非终态生成视频: generated_video_id={}, status={}",
                video.id, video.status
            )));
        }
        let file_path = context
            .media_file_writer
            .write_imported_media_file(
                context.project_id,
                CanvasNodeRefKind::GeneratedVideo.as_str(),
                video.id,
                FILE_PATH_FIELD,
                "videos",
                "imported_package_video",
            )
            .map_err(VideoStudioProjectImportError::MediaWrite)?;
        let thumbnail_path = context
            .media_file_writer
            .write_imported_media_file(
                context.project_id,
                CanvasNodeRefKind::GeneratedVideo.as_str(),
                video.id,
                THUMBNAIL_PATH_FIELD,
                "videos",
                "imported_package_video_thumbnail",
            )
            .map_err(VideoStudioProjectImportError::MediaWrite)?;
        let row = generated_video::ActiveModel {
            project_id: Set(context.project_id),
            chapter_id: Set(None),
            video_type: Set(video.video_type.clone()),
            workflow_mode: Set(video.workflow_mode.clone()),
            file_path: Set(file_path.clone()),
            thumbnail_path: Set(thumbnail_path.clone()),
            prompt: Set(video.prompt.clone()),
            negative_prompt: Set(video.negative_prompt.clone()),
            model: Set(video.model.clone()),
            width: Set(video.width),
            height: Set(video.height),
            duration_secs: Set(video.duration_secs),
            fps: Set(video.fps),
            file_size: Set(video.file_size),
            source_video_id: Set(None),
            source_image_id: Set(None),
            status: Set(video.status.clone()),
            provider_job_id: Set(None),
            error_message: Set(video.error_message.clone()),
            metadata_json: Set(video.metadata_json.clone()),
            is_favorite: Set(video.is_favorite),
            created_at: Set(context.now),
            updated_at: Set(context.now),
            ..Default::default()
        }
        .insert(context.db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        context.maps.video_ids.insert(video.id, row.id);
        context.maps.video_file_paths.insert(video.id, file_path);
        context
            .maps
            .video_thumbnail_paths
            .insert(video.id, thumbnail_path);
        imported.insert(video.id, row);
    }
    Ok(imported)
}

pub(in crate::canvas::transfer) async fn update_imported_video_source_refs<C>(
    db: &C,
    source_videos: &[generated_video::Model],
    imported_videos: &HashMap<i64, generated_video::Model>,
    maps: &super::super::types::VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for source in source_videos {
        let Some(imported) = imported_videos.get(&source.id) else {
            continue;
        };
        let source_video_id = remap_video_studio_optional_id(
            &maps.video_ids,
            source.source_video_id,
            "generated_video.source_video_id",
        )?;
        let source_image_id = remap_video_studio_optional_id(
            &maps.image_ids,
            source.source_image_id,
            "generated_video.source_image_id",
        )?;
        if source_video_id == imported.source_video_id
            && source_image_id == imported.source_image_id
        {
            continue;
        }
        generated_video::ActiveModel {
            id: Set(imported.id),
            source_video_id: Set(source_video_id),
            source_image_id: Set(source_image_id),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}
