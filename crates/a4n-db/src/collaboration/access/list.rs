use a4n_core::models::project::novel_project;
use sea_orm::{ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use super::super::{
    AccessibleProjectListInput, AccessibleProjectsPage, CollaborationDbError, ProjectAccessScope,
    accessible_member_condition, member_root_ids, normalize_project_kind, validate_positive_i64,
};
use sea_orm::DatabaseConnection;

pub async fn list_accessible_projects(
    db: &DatabaseConnection,
    input: AccessibleProjectListInput,
) -> Result<AccessibleProjectsPage, CollaborationDbError> {
    let member_root_ids = member_root_ids(db, input.user_id).await?;
    let mut condition = match input.access_scope {
        ProjectAccessScope::Owned => {
            Condition::all().add(novel_project::Column::UserId.eq(input.user_id))
        }
        ProjectAccessScope::Collaborating => accessible_member_condition(&member_root_ids),
        ProjectAccessScope::All => {
            let mut condition =
                Condition::any().add(novel_project::Column::UserId.eq(input.user_id));
            if !member_root_ids.is_empty() {
                condition = condition
                    .add(novel_project::Column::Id.is_in(member_root_ids.clone()))
                    .add(novel_project::Column::RootProjectId.is_in(member_root_ids.clone()));
            }
            Condition::all().add(condition)
        }
    };
    if let Some(kind) = input.project_kind.as_deref() {
        condition =
            condition.add(novel_project::Column::ProjectKind.eq(normalize_project_kind(kind)?));
    }
    if let Some(root_project_id) = input.root_project_id {
        validate_positive_i64(root_project_id, "root_project_id")?;
        condition = condition.add(novel_project::Column::RootProjectId.eq(root_project_id));
    }

    let paginator = novel_project::Entity::find()
        .filter(condition)
        .order_by_desc(novel_project::Column::UpdatedAt)
        .paginate(db, input.limit);
    let total = paginator
        .num_items()
        .await
        .map_err(CollaborationDbError::Database)?;
    let items = paginator
        .fetch_page(input.offset / input.limit)
        .await
        .map_err(CollaborationDbError::Database)?;
    Ok(AccessibleProjectsPage { items, total })
}
