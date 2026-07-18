use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_video, chapter::chapter_manga_prompt};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(crate) async fn clear_generated_image_references_record<C>(
    db: &C,
    image_ids: &[i64],
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    if image_ids.is_empty() {
        return Ok(());
    }

    let panels = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::GeneratedImageId.is_in(image_ids.iter().copied()))
        .all(db)
        .await?;
    for panel in panels {
        let mut active: chapter_manga_prompt::ActiveModel = panel.into();
        active.generated_image_id = Set(None);
        active.status = Set("pending".to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await?;
    }

    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::SourceImageId.is_in(image_ids.iter().copied()))
        .all(db)
        .await?;
    for video in videos {
        let mut active: generated_video::ActiveModel = video.into();
        active.source_image_id = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await?;
    }

    crate::canvas::mark_canvas_generated_image_refs_missing(db, image_ids).await?;
    Ok(())
}
