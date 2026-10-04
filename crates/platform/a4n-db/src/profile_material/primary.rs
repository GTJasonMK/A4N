use a4n_db_models::models::{ai::generated_image, asset::profile_primary_material};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};

use super::{
    contracts::ProfileMaterialDbError,
    validation::{database, require_positive, require_text},
};

pub async fn upsert_profile_primary_material(
    db: &impl crate::DbExecutor,
    project_id: i64,
    profile_kind: &str,
    profile_id: i64,
    image_id: i64,
) -> Result<(), ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    let profile_id = require_positive(profile_id, "profile_id")?;
    let image_id = require_positive(image_id, "image_id")?;
    let profile_kind = require_text(profile_kind, "profile_kind")?;
    let tx = db
        .begin()
        .await
        .map_err(|source| database("开启档案 primary material 事务失败", source))?;
    crate::project::project_write_identity_for_update(&tx, project_id)
        .await
        .map_err(|source| database("获取项目媒体引用锁失败", source))?
        .ok_or_else(|| {
            ProfileMaterialDbError::InvalidArgument(format!("项目不存在: {project_id}"))
        })?;
    let image_exists = generated_image::Entity::find_by_id(image_id)
        .select_only()
        .column(generated_image::Column::Id)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .one(&tx)
        .await
        .map_err(|source| database("复核档案 primary material 图片失败", source))?
        .is_some();
    if !image_exists {
        return Err(ProfileMaterialDbError::InvalidArgument(format!(
            "图片 #{image_id} 不存在或不属于项目 {project_id}"
        )));
    }
    let existing = profile_primary_material::Entity::find()
        .filter(profile_primary_material::Column::ProjectId.eq(project_id))
        .filter(profile_primary_material::Column::ProfileKind.eq(profile_kind))
        .filter(profile_primary_material::Column::ProfileId.eq(profile_id))
        .one(&tx)
        .await
        .map_err(|source| database("读取档案 primary material 失败", source))?;
    let now = Utc::now().naive_utc();
    match existing {
        Some(row) => {
            let mut active: profile_primary_material::ActiveModel = row.into();
            active.primary_image_id = Set(image_id);
            active.updated_at = Set(now);
            active
                .update(&tx)
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
            .insert(&tx)
            .await
            .map_err(|source| database("插入档案 primary material 失败", source))?;
        }
    }
    tx.commit()
        .await
        .map_err(|source| database("提交档案 primary material 事务失败", source))?;
    Ok(())
}

pub async fn delete_profile_primary_material(
    db: &impl crate::DbExecutor,
    project_id: i64,
    profile_kind: &str,
    profile_id: i64,
) -> Result<(), ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    let profile_id = require_positive(profile_id, "profile_id")?;
    let profile_kind = require_text(profile_kind, "profile_kind")?;
    let tx = db
        .begin()
        .await
        .map_err(|source| database("开启档案 primary material 删除事务失败", source))?;
    crate::project::project_write_identity_for_update(&tx, project_id)
        .await
        .map_err(|source| database("获取项目媒体引用锁失败", source))?
        .ok_or_else(|| {
            ProfileMaterialDbError::InvalidArgument(format!("项目不存在: {project_id}"))
        })?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::ProjectId.eq(project_id))
        .filter(profile_primary_material::Column::ProfileKind.eq(profile_kind))
        .filter(profile_primary_material::Column::ProfileId.eq(profile_id))
        .exec(&tx)
        .await
        .map_err(|source| database("删除档案 primary material 失败", source))?;
    tx.commit()
        .await
        .map_err(|source| database("提交档案 primary material 删除事务失败", source))?;
    Ok(())
}
