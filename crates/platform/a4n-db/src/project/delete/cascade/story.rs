use a4n_db_models::models::{
    ai::system_settings,
    chapter::{chapter, chapter_outline, part_outline},
    project::import_review_queue,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::project as project_db;
use crate::project::delete::{
    GeneratedImageDeleteScope, ProjectDeleteDbError, delete_chapter_generation_outputs,
};

pub(super) async fn delete_project_story_data<C>(
    db: &C,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError>
where
    C: ConnectionTrait,
{
    delete_chapter_generation_outputs(db, project_id, GeneratedImageDeleteScope::Project).await?;
    chapter::Entity::delete_many()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    chapter_outline::Entity::delete_many()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    part_outline::Entity::delete_many()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;

    project_db::delete_project_blueprints(db, project_id, true).await?;
    project_db::delete_project_conversations(db, project_id).await?;
    import_review_queue::Entity::delete_many()
        .filter(import_review_queue::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    project_db::delete_project_status_history(db, project_id).await?;
    delete_project_prompt_override_settings(db, project_id).await?;
    Ok(())
}

async fn delete_project_prompt_override_settings(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError> {
    let key_prefix = system_settings::prompt_override_key_prefix(project_id);
    system_settings::Entity::delete_many()
        .filter(system_settings::Column::Key.starts_with(key_prefix))
        .exec(db)
        .await?;
    Ok(())
}
