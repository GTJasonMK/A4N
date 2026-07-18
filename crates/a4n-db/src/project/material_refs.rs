use super::ProjectFamilyDbError;
use a4n_core::models::{
    asset::profile_primary_material,
    character::{character_portrait, protagonist_profile},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
};

pub async fn character_portrait_project_id(
    db: &DatabaseConnection,
    portrait_id: i64,
) -> Result<i64, ProjectFamilyDbError> {
    if portrait_id <= 0 {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "portrait_id 必须大于 0".to_string(),
        ));
    }
    let portrait = character_portrait::Entity::find_by_id(portrait_id)
        .one(db)
        .await
        .map_err(ProjectFamilyDbError::Database)?
        .ok_or_else(|| ProjectFamilyDbError::NotFound(format!("立绘不存在: {portrait_id}")))?;
    Ok(portrait.project_id)
}

pub async fn protagonist_profile_project_id(
    db: &DatabaseConnection,
    profile_id: i64,
) -> Result<i64, ProjectFamilyDbError> {
    if profile_id <= 0 {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "profile_id 必须大于 0".to_string(),
        ));
    }
    let profile = protagonist_profile::Entity::find_by_id(profile_id)
        .one(db)
        .await
        .map_err(ProjectFamilyDbError::Database)?
        .ok_or_else(|| ProjectFamilyDbError::NotFound(format!("角色档案不存在: {profile_id}")))?;
    Ok(profile.project_id)
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
