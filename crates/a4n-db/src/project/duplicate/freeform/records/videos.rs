use a4n_core::models::ai::generated_video;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::project::duplicate::{
    ProjectDuplicateError, freeform::FreeformGeneratedVideoCopyRecords,
    shared::ProjectMediaFileCopier,
};

pub(super) async fn copy_freeform_generated_videos_record<C, Copier>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<FreeformGeneratedVideoCopyRecords, ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: ProjectMediaFileCopier,
{
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(source_project_id))
        .order_by_asc(generated_video::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut copied = FreeformGeneratedVideoCopyRecords::default();
    for video in videos {
        let file_path = media_file_copier
            .copy_media_file(
                new_project_id,
                video.id,
                "video",
                video.file_path.as_deref(),
            )
            .map_err(ProjectDuplicateError::MediaCopy)?;
        let thumbnail_path = media_file_copier
            .copy_media_file(
                new_project_id,
                video.id,
                "thumbnail",
                video.thumbnail_path.as_deref(),
            )
            .map_err(ProjectDuplicateError::MediaCopy)?;
        let row = generated_video::ActiveModel {
            project_id: Set(new_project_id),
            chapter_id: Set(None),
            video_type: Set(video.video_type),
            workflow_mode: Set(video.workflow_mode),
            file_path: Set(file_path.clone()),
            thumbnail_path: Set(thumbnail_path.clone()),
            prompt: Set(video.prompt),
            negative_prompt: Set(video.negative_prompt),
            model: Set(video.model),
            width: Set(video.width),
            height: Set(video.height),
            duration_secs: Set(video.duration_secs),
            fps: Set(video.fps),
            file_size: Set(video.file_size),
            source_video_id: Set(None),
            source_image_id: Set(None),
            status: Set(video.status),
            provider_job_id: Set(video.provider_job_id),
            error_message: Set(video.error_message),
            metadata_json: Set(video.metadata_json),
            is_favorite: Set(video.is_favorite),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        copied.file_paths.insert(video.id, file_path);
        copied.thumbnail_paths.insert(video.id, thumbnail_path);
        copied.ids.insert(video.id, row.id);
    }
    Ok(copied)
}
