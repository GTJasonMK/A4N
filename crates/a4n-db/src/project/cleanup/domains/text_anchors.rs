use super::super::{
    ProjectCleanupDbError, ProjectTextAnchorCleanupResult, ProjectTextAnchorDeleteScope,
};
use crate::project::validate_positive_i64;
use a4n_core::models::project::{profile_evidence, text_anchor};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub async fn delete_project_text_anchors<C>(
    db: &C,
    project_id: i64,
    scope: ProjectTextAnchorDeleteScope,
) -> Result<ProjectTextAnchorCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let mut anchor_query = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::Id)
        .filter(text_anchor::Column::ProjectId.eq(project_id));
    if matches!(scope, ProjectTextAnchorDeleteScope::CurrentProjectContent) {
        anchor_query = anchor_query.filter(text_anchor::Column::SourceSnapshotId.is_null());
    }
    let text_anchor_ids = anchor_query.into_tuple::<i64>().all(db).await?;

    let deleted_profile_evidence = if text_anchor_ids.is_empty() {
        0
    } else {
        profile_evidence::Entity::delete_many()
            .filter(profile_evidence::Column::TextAnchorId.is_in(text_anchor_ids.clone()))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_text_anchors = if text_anchor_ids.is_empty() {
        0
    } else {
        text_anchor::Entity::delete_many()
            .filter(text_anchor::Column::Id.is_in(text_anchor_ids))
            .exec(db)
            .await?
            .rows_affected
    };

    Ok(ProjectTextAnchorCleanupResult {
        deleted_text_anchors,
        deleted_profile_evidence,
    })
}
