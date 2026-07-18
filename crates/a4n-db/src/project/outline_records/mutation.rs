use a4n_core::models::chapter::chapter_outline;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use super::contracts::{ProjectChapterOutlineUpdateError, ProjectChapterOutlineUpdateInput};

pub async fn update_chapter_outline_record<C>(
    db: &C,
    input: ProjectChapterOutlineUpdateInput,
) -> Result<chapter_outline::Model, ProjectChapterOutlineUpdateError>
where
    C: ConnectionTrait,
{
    let outline = chapter_outline::Entity::find_by_id(input.outline_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectChapterOutlineUpdateError::NotFound(format!(
                "章节大纲不存在: {}",
                input.outline_id
            ))
        })?;
    if outline.project_id != input.project_id {
        return Err(ProjectChapterOutlineUpdateError::Forbidden(format!(
            "章节大纲 {} 不属于项目 {}",
            outline.id, input.project_id
        )));
    }

    let mut active: chapter_outline::ActiveModel = outline.into();
    if let Some(title) = input.title {
        active.title = Set(Some(title));
    }
    if let Some(summary) = input.summary {
        active.summary = Set(Some(summary));
    }
    if let Some(beats) = input.beats {
        active.beats = Set(Some(beats));
    }
    if let Some(key_scenes) = input.key_scenes {
        active.key_scenes = Set(Some(key_scenes));
    }
    active.updated_at = Set(Utc::now().naive_utc());

    active
        .update(db)
        .await
        .map_err(ProjectChapterOutlineUpdateError::Database)
}
