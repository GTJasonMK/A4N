use super::super::{ProjectCleanupDbError, ProjectTaskQueueCleanupResult};
use crate::project::validate_positive_i64;
use a4n_db_models::models::{
    generation::generation_start_request,
    task::{task_dependency, task_queue},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub(crate) async fn delete_project_task_queue_records<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectTaskQueueCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let task_ids = task_queue::Entity::find()
        .select_only()
        .column(task_queue::Column::Id)
        .filter(task_queue::Column::ProjectId.eq(project_id))
        .into_tuple::<String>()
        .all(db)
        .await?;

    let mut deleted_task_dependencies = 0;
    if !task_ids.is_empty() {
        generation_start_request::Entity::delete_many()
            .filter(generation_start_request::Column::TaskId.is_in(task_ids.clone()))
            .exec(db)
            .await?;
        deleted_task_dependencies += task_dependency::Entity::delete_many()
            .filter(task_dependency::Column::TaskId.is_in(task_ids.clone()))
            .exec(db)
            .await?
            .rows_affected;
        deleted_task_dependencies += task_dependency::Entity::delete_many()
            .filter(task_dependency::Column::DependsOnTaskId.is_in(task_ids))
            .exec(db)
            .await?
            .rows_affected;
    }
    let deleted_tasks = task_queue::Entity::delete_many()
        .filter(task_queue::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;

    Ok(ProjectTaskQueueCleanupResult {
        deleted_tasks,
        deleted_task_dependencies,
    })
}
