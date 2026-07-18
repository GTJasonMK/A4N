use a4n_core::models::asset::profile_primary_material;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use super::{
    contracts::ProfileMaterialDbError,
    validation::{database, require_positive, require_text},
};

pub async fn upsert_profile_primary_material(
    db: &DatabaseConnection,
    project_id: i64,
    profile_kind: &str,
    profile_id: i64,
    image_id: i64,
) -> Result<(), ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    let profile_id = require_positive(profile_id, "profile_id")?;
    let image_id = require_positive(image_id, "image_id")?;
    let profile_kind = require_text(profile_kind, "profile_kind")?;
    let existing = profile_primary_material::Entity::find()
        .filter(profile_primary_material::Column::ProjectId.eq(project_id))
        .filter(profile_primary_material::Column::ProfileKind.eq(profile_kind))
        .filter(profile_primary_material::Column::ProfileId.eq(profile_id))
        .one(db)
        .await
        .map_err(|source| database("读取档案 primary material 失败", source))?;
    let now = Utc::now().naive_utc();
    match existing {
        Some(row) => {
            let mut active: profile_primary_material::ActiveModel = row.into();
            active.primary_image_id = Set(image_id);
            active.updated_at = Set(now);
            active
                .update(db)
                .await
                .map_err(|source| database("更新档案 primary material 失败", source))?;
        }
        None => {
            profile_primary_material::ActiveModel {
                project_id: Set(project_id),
                profile_kind: Set(profile_kind.to_string()),
                profile_id: Set(profile_id),
                primary_image_id: Set(image_id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|source| database("插入档案 primary material 失败", source))?;
        }
    }
    Ok(())
}

pub async fn delete_profile_primary_material(
    db: &DatabaseConnection,
    project_id: i64,
    profile_kind: &str,
    profile_id: i64,
) -> Result<(), ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    let profile_id = require_positive(profile_id, "profile_id")?;
    let profile_kind = require_text(profile_kind, "profile_kind")?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::ProjectId.eq(project_id))
        .filter(profile_primary_material::Column::ProfileKind.eq(profile_kind))
        .filter(profile_primary_material::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(|source| database("删除档案 primary material 失败", source))?;
    Ok(())
}
