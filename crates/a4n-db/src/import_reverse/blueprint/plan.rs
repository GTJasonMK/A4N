use a4n_core::models::project::{novel_blueprint, novel_project};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::import_reverse::{ImportReverseDbError, ReverseBlueprintApplyPlanRecord};

pub async fn reverse_blueprint_apply_plan_record<C>(
    db: &C,
    project_id: i64,
) -> Result<ReverseBlueprintApplyPlanRecord, ImportReverseDbError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(ImportReverseDbError::Database)?
        .ok_or_else(|| {
            ImportReverseDbError::NotFound(format!("项目不存在: project_id={project_id}"))
        })?;
    let existing_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await?;

    Ok(ReverseBlueprintApplyPlanRecord {
        project_total_chapters: project.total_chapters.filter(|total| *total > 0),
        existing_blueprint_total_chapters: existing_blueprint
            .as_ref()
            .and_then(|blueprint| blueprint.total_chapters.filter(|total| *total > 0)),
    })
}
