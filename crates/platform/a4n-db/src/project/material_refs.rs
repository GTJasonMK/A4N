use super::ProjectFamilyDbError;
use a4n_db_models::models::{asset::profile_primary_material, character::protagonist_profile};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub async fn protagonist_profile_project_id(
    db: &impl crate::DbExecutor,
    profile_id: i64,
) -> Result<i64, ProjectFamilyDbError> {
    if profile_id <= 0 {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "profile_id 必须大于 0".to_string(),
        ));
    }
    let profile = protagonist_profile::Entity::find_by_id(profile_id)
        .select_only()
        .column(protagonist_profile::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(ProjectFamilyDbError::Database)?
        .ok_or_else(|| ProjectFamilyDbError::NotFound(format!("角色档案不存在: {profile_id}")))?;
    Ok(profile)
}

pub async fn primary_material_image_ids_for_projects<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<i64>, ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    if project_ids.iter().any(|project_id| *project_id <= 0) {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "project_ids 必须全部大于 0".to_string(),
        ));
    }
    let mut project_ids = project_ids.to_vec();
    project_ids.sort_unstable();
    project_ids.dedup();
    profile_primary_material::Entity::find()
        .select_only()
        .column(profile_primary_material::Column::PrimaryImageId)
        .filter(profile_primary_material::Column::ProjectId.is_in(project_ids))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectFamilyDbError::Database)
}
