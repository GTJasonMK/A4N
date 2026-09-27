use a4n_db_models::models::project::{
    novel_project, project_invite, project_member, project_source_snapshot,
    source_snapshot_chapter, source_snapshot_outline, source_snapshot_profile,
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, QueryTrait,
};

use crate::project as project_db;
use crate::project::delete::ProjectDeleteDbError;

pub(super) async fn delete_project_final_records<C>(
    db: &C,
    project_id: i64,
    source_snapshot_id: Option<i64>,
) -> Result<(), ProjectDeleteDbError>
where
    C: ConnectionTrait,
{
    let mut snapshot_scope =
        Condition::any().add(project_source_snapshot::Column::RootProjectId.eq(project_id));
    if let Some(id) = source_snapshot_id {
        snapshot_scope = snapshot_scope.add(project_source_snapshot::Column::Id.eq(id));
    }
    let snapshots = project_source_snapshot::Entity::find()
        .select_only()
        .column(project_source_snapshot::Column::Id)
        .filter(snapshot_scope.clone())
        .into_query();
    source_snapshot_profile::Entity::delete_many()
        .filter(source_snapshot_profile::Column::SnapshotId.in_subquery(snapshots.clone()))
        .exec(db)
        .await?;
    source_snapshot_outline::Entity::delete_many()
        .filter(source_snapshot_outline::Column::SnapshotId.in_subquery(snapshots.clone()))
        .exec(db)
        .await?;
    source_snapshot_chapter::Entity::delete_many()
        .filter(source_snapshot_chapter::Column::SnapshotId.in_subquery(snapshots))
        .exec(db)
        .await?;
    project_source_snapshot::Entity::delete_many()
        .filter(snapshot_scope)
        .exec(db)
        .await?;

    project_db::delete_project_optimization_tree(db, project_id).await?;
    project_db::delete_project_task_queue_records(db, project_id).await?;
    project_invite::Entity::delete_many()
        .filter(project_invite::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    project_member::Entity::delete_many()
        .filter(project_member::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    novel_project::Entity::delete_by_id(project_id)
        .exec(db)
        .await?;
    Ok(())
}
