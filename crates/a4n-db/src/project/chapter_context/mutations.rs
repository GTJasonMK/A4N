use super::contracts::{ProjectChapterContextError, ProtagonistProfileCurrentStateUpdate};
use a4n_core::models::{chapter::chapter, character::protagonist_profile, project::novel_project};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set,
};

pub async fn refresh_project_current_chapter(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<(), ProjectChapterContextError> {
    let max_chapter = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_desc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)?
        .into_iter()
        .find(chapter::Model::has_available_content)
        .map_or(0, |chapter| chapter.chapter_number);

    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(ProjectChapterContextError::Database)?
        .ok_or_else(|| {
            ProjectChapterContextError::NotFound(format!("项目不存在: project_id={project_id}"))
        })?;
    if project.current_chapter == max_chapter {
        return Ok(());
    }

    let mut active: novel_project::ActiveModel = project.into();
    active.current_chapter = Set(max_chapter);
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    Ok(())
}

pub async fn update_protagonist_profile_current_state<C>(
    db: &C,
    profile: protagonist_profile::Model,
    fields: ProtagonistProfileCurrentStateUpdate,
    last_synced_chapter: i32,
) -> Result<(), ProjectChapterContextError>
where
    C: ConnectionTrait,
{
    let mut active: protagonist_profile::ActiveModel = profile.into();
    active.gender = Set(fields.gender);
    active.age = Set(fields.age);
    active.appearance = Set(fields.appearance);
    active.background = Set(fields.background);
    active.personality = Set(fields.personality);
    active.current_state = Set(fields.current_state);
    active.last_synced_chapter = Set(last_synced_chapter);
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    Ok(())
}

pub async fn delete_chapter_content_record<C>(
    db: &C,
    chapter_id: i64,
) -> Result<(), ProjectChapterContextError>
where
    C: ConnectionTrait,
{
    chapter::Entity::delete_by_id(chapter_id)
        .exec(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    Ok(())
}
