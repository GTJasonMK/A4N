use a4n_core::models::project::novel_project;
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use super::super::{
    AccessibleProjectListInput, AccessibleProjectRecord, AccessibleProjectsPage,
    CollaborationDbError, ProjectAccessScope, accessible_member_condition, member_root_ids,
    validate_positive_i64,
};
use sea_orm::DatabaseConnection;

pub async fn list_accessible_projects(
    db: &DatabaseConnection,
    input: AccessibleProjectListInput,
) -> Result<AccessibleProjectsPage, CollaborationDbError> {
    let mut condition = accessible_project_condition(db, input.user_id, input.access_scope).await?;
    if let Some(kind) = input.project_kind {
        condition = condition.add(novel_project::Column::ProjectKind.eq(kind.as_str()));
    }
    if let Some(root_project_id) = input.root_project_id {
        validate_positive_i64(root_project_id, "root_project_id")?;
        condition = condition.add(novel_project::Column::RootProjectId.eq(root_project_id));
    }

    let paginator = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::UserId)
        .column(novel_project::Column::Title)
        .column(novel_project::Column::Description)
        .column(novel_project::Column::ProjectType)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .column(novel_project::Column::SourceSyncStatus)
        .column(novel_project::Column::Status)
        .column(novel_project::Column::LifecycleGate)
        .column(novel_project::Column::TargetChapters)
        .column(novel_project::Column::CurrentChapter)
        .column(novel_project::Column::TotalChapters)
        .column(novel_project::Column::ChaptersPerPart)
        .column(novel_project::Column::CreatedAt)
        .column(novel_project::Column::UpdatedAt)
        .filter(condition)
        .order_by_desc(novel_project::Column::UpdatedAt)
        .into_model::<AccessibleProjectRecord>()
        .paginate(db, input.limit);
    let (total, items) = futures::join!(
        paginator.num_items(),
        paginator.fetch_page(input.offset / input.limit),
    );
    let total = total.map_err(CollaborationDbError::Database)?;
    let items = items.map_err(CollaborationDbError::Database)?;
    Ok(AccessibleProjectsPage { items, total })
}

/// 返回当前用户按协作规则可见的项目 ID，供其他 DB 查询在分页前做授权过滤。
pub async fn accessible_project_ids(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Vec<i64>, CollaborationDbError> {
    novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .filter(accessible_project_condition(db, user_id, ProjectAccessScope::All).await?)
        .order_by_asc(novel_project::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)
}

async fn accessible_project_condition(
    db: &DatabaseConnection,
    user_id: i64,
    access_scope: ProjectAccessScope,
) -> Result<Condition, CollaborationDbError> {
    let member_root_ids = member_root_ids(db, user_id).await?;
    Ok(match access_scope {
        ProjectAccessScope::Owned => {
            Condition::all().add(novel_project::Column::UserId.eq(user_id))
        }
        ProjectAccessScope::Collaborating => accessible_member_condition(&member_root_ids),
        ProjectAccessScope::All => {
            let mut condition = Condition::any().add(novel_project::Column::UserId.eq(user_id));
            if !member_root_ids.is_empty() {
                condition = condition
                    .add(novel_project::Column::Id.is_in(member_root_ids.clone()))
                    .add(novel_project::Column::RootProjectId.is_in(member_root_ids));
            }
            Condition::all().add(condition)
        }
    })
}
