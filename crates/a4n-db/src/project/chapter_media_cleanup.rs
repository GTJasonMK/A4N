use super::{ProjectCleanupDbError, project_by_id_for_update};
use a4n_core::models::{
    ai::{generated_image, generated_video},
    asset::profile_primary_material,
    chapter::{chapter, chapter_manga_prompt, chapter_scene, chapter_video_prompt},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

pub async fn cleanup_chapter_media_data<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<String>, ProjectCleanupDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await?;
    let paths = cleanup_chapter_media_data_in(&txn, project_id, chapter_id).await?;
    txn.commit().await?;
    Ok(paths)
}

async fn cleanup_chapter_media_data_in<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<String>, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    project_by_id_for_update(db, project_id)
        .await
        .map_err(ProjectCleanupDbError::from)?
        .ok_or_else(|| ProjectCleanupDbError::NotFound(format!("项目不存在: {project_id}")))?;
    let mut chapter_query = chapter::Entity::find_by_id(chapter_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        chapter_query = chapter_query.lock_exclusive();
    }
    let chapter = chapter_query
        .one(db)
        .await
        .map_err(ProjectCleanupDbError::from)?
        .ok_or_else(|| {
            ProjectCleanupDbError::NotFound(format!("章节不存在: chapter_id={chapter_id}"))
        })?;
    if chapter.project_id != project_id {
        return Err(ProjectCleanupDbError::InvalidState {
            current: format!("chapter_project_id={}", chapter.project_id),
            operation: format!("清理项目 {project_id} 的章节 {chapter_id} 媒体数据"),
        });
    }

    let mut image_query = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_asc(generated_image::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        image_query = image_query.lock_exclusive();
    }
    let images = image_query
        .all(db)
        .await
        .map_err(ProjectCleanupDbError::from)?;
    let image_files = images
        .iter()
        .map(|image| image.file_path.clone())
        .collect::<Vec<_>>();
    let image_ids = images.iter().map(|image| image.id).collect::<Vec<_>>();
    let mut video_query = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .filter(generated_video::Column::ChapterId.eq(chapter_id))
        .order_by_asc(generated_video::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        video_query = video_query.lock_exclusive();
    }
    let videos = video_query
        .all(db)
        .await
        .map_err(ProjectCleanupDbError::from)?;
    let video_files = videos
        .iter()
        .flat_map(|video| [video.file_path.clone(), video.thumbnail_path.clone()])
        .flatten()
        .collect::<Vec<_>>();
    let video_ids = videos.iter().map(|video| video.id).collect::<Vec<_>>();
    let manga_prompt_ids = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::Id)
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectCleanupDbError::from)?;
    let video_prompt_ids = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectCleanupDbError::from)?;

    crate::video::clear_generated_image_references_record(db, &image_ids).await?;
    if !image_ids.is_empty() {
        profile_primary_material::Entity::delete_many()
            .filter(profile_primary_material::Column::PrimaryImageId.is_in(image_ids.clone()))
            .exec(db)
            .await?;
    }
    crate::video::clear_generated_video_references_record(db, &videos).await?;
    crate::media::delete_generated_image_target_media_bindings(db, &image_ids).await?;
    crate::media::delete_generated_video_target_media_bindings(db, &video_ids).await?;

    crate::generation::delete_generation_run_targets_for(
        db,
        "chapter_manga_prompt",
        &manga_prompt_ids,
    )
    .await?;
    crate::generation::delete_generation_run_targets_for(
        db,
        "chapter_video_prompt",
        &video_prompt_ids,
    )
    .await?;
    crate::generation::delete_generation_run_targets_for(db, "generated_image", &image_ids).await?;
    crate::generation::delete_generation_run_targets_for(db, "generated_video", &video_ids).await?;
    chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .exec(db)
        .await?;
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .exec(db)
        .await?;
    chapter_scene::Entity::delete_many()
        .filter(chapter_scene::Column::ChapterId.eq(chapter_id))
        .exec(db)
        .await?;
    if !image_ids.is_empty() {
        generated_image::Entity::delete_many()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::Id.is_in(image_ids))
            .exec(db)
            .await?;
    }
    if !video_ids.is_empty() {
        generated_video::Entity::delete_many()
            .filter(generated_video::Column::ProjectId.eq(project_id))
            .filter(generated_video::Column::Id.is_in(video_ids))
            .exec(db)
            .await?;
    }

    let candidate_paths = image_files
        .into_iter()
        .chain(video_files)
        .collect::<Vec<_>>();
    crate::media::filter_unreferenced_media_paths(db, &candidate_paths)
        .await
        .map_err(ProjectCleanupDbError::from)
}
