use super::contracts::ProjectMutationError;
use a4n_core::models::project::novel_project;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, QuerySelect};

pub async fn project_by_id_record<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<novel_project::Model>, ProjectMutationError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(ProjectMutationError::Database)
}

pub(crate) async fn project_by_id_for_update<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<novel_project::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = novel_project::Entity::find_by_id(project_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query.one(db).await
}

pub async fn project_ids(db: &DatabaseConnection) -> Result<Vec<i64>, ProjectMutationError> {
    novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)
}
