use super::super::{
    remap::remap_video_studio_optional_id,
    types::{VideoStudioProjectImportError, VideoStudioProjectImportMediaFileWriter},
};
use super::VideoStudioProjectImportMediaRowsContext;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_domain::CanvasNodeRefKind;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;

pub(in crate::canvas::transfer) async fn import_video_studio_generated_images<C, Writer>(
    context: &mut VideoStudioProjectImportMediaRowsContext<'_, C, Writer>,
    images: &[generated_image::Model],
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    let mut rows = Vec::with_capacity(images.len());
    let mut file_paths = Vec::with_capacity(images.len());
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
        rows.push(generated_image::ActiveModel {
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
        });
        file_paths.push(target_relative);
    }
    if rows.is_empty() {
        return Ok(());
    }
    let inserted = generated_image::Entity::insert_many(rows)
        .exec_with_returning_many(context.db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if inserted.len() != images.len() {
        return Err(VideoStudioProjectImportError::InvalidPackage(format!(
            "生成图片批量写入返回行数不一致: inserted={}, expected={}",
            inserted.len(),
            images.len()
        )));
    }
    for ((image, file_path), row) in images.iter().zip(file_paths).zip(inserted) {
        context.maps.image_ids.insert(image.id, row.id);
        context.maps.image_file_paths.insert(image.id, file_path);
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
    let mut rows = Vec::with_capacity(videos.len());
    let mut file_paths = Vec::with_capacity(videos.len());
    let mut thumbnail_paths = Vec::with_capacity(videos.len());
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
        rows.push(generated_video::ActiveModel {
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
        });
        file_paths.push(file_path);
        thumbnail_paths.push(thumbnail_path);
    }
    if rows.is_empty() {
        return Ok(imported);
    }
    let inserted = generated_video::Entity::insert_many(rows)
        .exec_with_returning_many(context.db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if inserted.len() != videos.len() {
        return Err(VideoStudioProjectImportError::InvalidPackage(format!(
            "生成视频批量写入返回行数不一致: inserted={}, expected={}",
            inserted.len(),
            videos.len()
        )));
    }
    for (((video, file_path), thumbnail_path), row) in videos
        .iter()
        .zip(file_paths)
        .zip(thumbnail_paths)
        .zip(inserted)
    {
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
    let mut source_video_id_case = CaseStatement::new();
    let mut source_image_id_case = CaseStatement::new();
    let mut updated_ids = Vec::new();
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
        source_video_id_case = source_video_id_case.case(
            generated_video::Column::Id.eq(imported.id),
            Expr::value(source_video_id),
        );
        source_image_id_case = source_image_id_case.case(
            generated_video::Column::Id.eq(imported.id),
            Expr::value(source_image_id),
        );
        updated_ids.push(imported.id);
    }
    if updated_ids.is_empty() {
        return Ok(());
    }
    let updated = generated_video::Entity::update_many()
        .col_expr(
            generated_video::Column::SourceVideoId,
            source_video_id_case
                .finally(Expr::col(generated_video::Column::SourceVideoId))
                .into(),
        )
        .col_expr(
            generated_video::Column::SourceImageId,
            source_image_id_case
                .finally(Expr::col(generated_video::Column::SourceImageId))
                .into(),
        )
        .col_expr(generated_video::Column::UpdatedAt, Expr::value(now))
        .filter(generated_video::Column::Id.is_in(updated_ids.clone()))
        .exec(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if updated.rows_affected != updated_ids.len() as u64 {
        return Err(VideoStudioProjectImportError::InvalidPackage(format!(
            "导入生成视频来源引用更新数量不一致: updated={}, expected={}",
            updated.rows_affected,
            updated_ids.len()
        )));
    }
    Ok(())
}
