use a4n_core::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter_manga_prompt, chapter_scene, chapter_video_prompt},
};
use a4n_services::generation::delete_generation_run_targets_for;
use a4n_services::video::mark_canvas_generated_video_refs_missing;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub(super) async fn cleanup_chapter_media_data_in<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<String>, String>
where
    C: ConnectionTrait,
{
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .all(db)
        .await
        .map_err(|err| format!("查询章节图片文件失败: {err}"))?;
    let image_files = images
        .iter()
        .map(|image| image.file_path.clone())
        .collect::<Vec<_>>();
    let image_ids = images.iter().map(|image| image.id).collect::<Vec<_>>();
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .filter(generated_video::Column::ChapterId.eq(chapter_id))
        .all(db)
        .await
        .map_err(|err| format!("查询章节视频文件失败: {err}"))?;
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
        .map_err(|err| format!("查询章节漫画提示词失败: {err}"))?;
    let video_prompt_ids = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(|err| format!("查询章节视频提示词失败: {err}"))?;

    delete_generation_run_targets_for(db, "chapter_manga_prompt", &manga_prompt_ids)
        .await
        .map_err(|err| err.to_string())?;
    delete_generation_run_targets_for(db, "chapter_video_prompt", &video_prompt_ids)
        .await
        .map_err(|err| err.to_string())?;
    delete_generation_run_targets_for(db, "generated_image", &image_ids)
        .await
        .map_err(|err| err.to_string())?;
    delete_generation_run_targets_for(db, "generated_video", &video_ids)
        .await
        .map_err(|err| err.to_string())?;
    for video_id in &video_ids {
        mark_canvas_generated_video_refs_missing(db, project_id, *video_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .exec(db)
        .await
        .map_err(|err| format!("清理章节漫画提示词失败: {err}"))?;
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .exec(db)
        .await
        .map_err(|err| format!("清理章节视频提示词失败: {err}"))?;
    chapter_scene::Entity::delete_many()
        .filter(chapter_scene::Column::ChapterId.eq(chapter_id))
        .exec(db)
        .await
        .map_err(|err| format!("清理章节场景失败: {err}"))?;
    generated_image::Entity::delete_many()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .exec(db)
        .await
        .map_err(|err| format!("清理章节图片记录失败: {err}"))?;
    generated_video::Entity::delete_many()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .filter(generated_video::Column::ChapterId.eq(chapter_id))
        .exec(db)
        .await
        .map_err(|err| format!("清理章节视频记录失败: {err}"))?;

    Ok(image_files.into_iter().chain(video_files).collect())
}
