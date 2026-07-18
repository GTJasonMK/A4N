use a4n_core::models::project::{
    novel_project, project_derivation, project_invite, project_member, project_source_snapshot,
    source_snapshot_chapter, source_snapshot_outline, source_snapshot_profile,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::project as project_db;
use crate::project_delete::ProjectDeleteDbError;

pub(super) async fn delete_project_final_records<C>(
    db: &C,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError>
where
    C: ConnectionTrait,
{
    let derivations = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.eq(project_id))
        .all(db)
        .await?;
    project_derivation::Entity::delete_many()
        .filter(project_derivation::Column::DerivedProjectId.eq(project_id))
        .exec(db)
        .await?;
    for derivation in derivations {
        source_snapshot_profile::Entity::delete_many()
            .filter(source_snapshot_profile::Column::SnapshotId.eq(derivation.source_snapshot_id))
            .exec(db)
            .await?;
        source_snapshot_outline::Entity::delete_many()
            .filter(source_snapshot_outline::Column::SnapshotId.eq(derivation.source_snapshot_id))
            .exec(db)
            .await?;
        source_snapshot_chapter::Entity::delete_many()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(derivation.source_snapshot_id))
            .exec(db)
            .await?;
        project_source_snapshot::Entity::delete_by_id(derivation.source_snapshot_id)
            .exec(db)
            .await?;
    }

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
