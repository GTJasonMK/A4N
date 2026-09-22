use super::*;

pub(crate) async fn delete_chapter_generation_outputs<C>(
    db: &C,
    project_id: i64,
    image_scope: GeneratedImageDeleteScope,
) -> Result<ChapterGenerationCleanupResult, ProjectDeleteDbError>
where
    C: ConnectionTrait,
{
    use a4n_db_models::models::ai::{generated_image, generated_video};
    use a4n_db_models::models::asset::profile_primary_material;
    use a4n_db_models::models::chapter::{
        chapter, chapter_manga_prompt, chapter_scene, chapter_video_prompt,
    };

    project_db::project_write_identity_for_update(db, project_id)
        .await
        .map_err(ProjectDeleteDbError::from)?
        .ok_or(ProjectDeleteDbError::ProjectNotFound { project_id })?;

    let chapter_ids = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .into_query();
    let manga_prompt_ids = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::Id)
        .filter(chapter_manga_prompt::Column::ChapterId.in_subquery(chapter_ids.clone()))
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let video_prompt_ids = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let mut video_query = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Id)
        .column(generated_video::Column::ProjectId)
        .column(generated_video::Column::FilePath)
        .column(generated_video::Column::ThumbnailPath)
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_video::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        video_query = video_query.lock_exclusive();
    }
    let generated_video_rows = video_query
        .into_tuple::<(i64, i64, Option<String>, Option<String>)>()
        .all(db)
        .await?;
    let generated_video_ids = generated_video_rows
        .iter()
        .map(|(video_id, _, _, _)| *video_id)
        .collect::<Vec<_>>();
    let generated_video_refs = generated_video_rows
        .iter()
        .map(|(video_id, project_id, _, _)| (*video_id, *project_id))
        .collect::<Vec<_>>();
    let image_query =
        generated_image::Entity::find().filter(generated_image::Column::ProjectId.eq(project_id));
    let mut image_query = match image_scope {
        GeneratedImageDeleteScope::Project => image_query,
        GeneratedImageDeleteScope::Chapters => {
            image_query.filter(generated_image::Column::ChapterId.in_subquery(chapter_ids.clone()))
        }
    };
    image_query = image_query.order_by_asc(generated_image::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        image_query = image_query.lock_exclusive();
    }
    let generated_image_rows = image_query
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::FilePath)
        .into_tuple::<(i64, String)>()
        .all(db)
        .await?;
    let generated_image_ids = generated_image_rows
        .iter()
        .map(|(image_id, _)| *image_id)
        .collect::<Vec<_>>();
    let mut candidate_media_paths = generated_image_rows
        .iter()
        .map(|(_, file_path)| file_path.clone())
        .chain(
            generated_video_rows
                .iter()
                .flat_map(|(_, _, file_path, thumbnail_path)| {
                    [file_path.clone(), thumbnail_path.clone()]
                })
                .flatten(),
        )
        .filter(|path| !path.trim().is_empty())
        .collect::<Vec<_>>();
    candidate_media_paths.sort();
    candidate_media_paths.dedup();

    crate::chapter_visual_plan::ensure_visual_plan_images_unreferenced(db, &generated_image_ids)
        .await?;
    crate::video::clear_generated_image_references_record(db, &generated_image_ids).await?;
    if !generated_image_ids.is_empty() {
        profile_primary_material::Entity::delete_many()
            .filter(
                profile_primary_material::Column::PrimaryImageId
                    .is_in(generated_image_ids.iter().copied()),
            )
            .exec(db)
            .await?;
    }
    crate::video::clear_generated_video_references_by_ids(db, &generated_video_refs).await?;
    crate::media::delete_generated_image_target_media_bindings(db, &generated_image_ids).await?;
    crate::media::delete_generated_video_target_media_bindings(db, &generated_video_ids).await?;

    generation::delete_generation_run_targets_for(db, "chapter_manga_prompt", &manga_prompt_ids)
        .await?;
    generation::delete_generation_run_targets_for(db, "chapter_video_prompt", &video_prompt_ids)
        .await?;
    generation::delete_generation_run_targets_for(db, "generated_video", &generated_video_ids)
        .await?;
    generation::delete_generation_run_targets_for(db, "generated_image", &generated_image_ids)
        .await?;
    let deleted_manga_prompts = chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::ChapterId.in_subquery(chapter_ids.clone()))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_video_prompts = chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_chapter_scenes = chapter_scene::Entity::delete_many()
        .filter(chapter_scene::Column::ChapterId.in_subquery(chapter_ids.clone()))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_generated_videos = if generated_video_ids.is_empty() {
        0
    } else {
        generated_video::Entity::delete_many()
            .filter(generated_video::Column::ProjectId.eq(project_id))
            .filter(generated_video::Column::Id.is_in(generated_video_ids))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_generated_images = if generated_image_ids.is_empty() {
        0
    } else {
        generated_image::Entity::delete_many()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::Id.is_in(generated_image_ids))
            .exec(db)
            .await?
            .rows_affected
    };

    Ok(ChapterGenerationCleanupResult {
        deleted_manga_prompts,
        deleted_video_prompts,
        deleted_chapter_scenes,
        deleted_generated_images,
        deleted_generated_videos,
        candidate_media_paths,
    })
}
