use a4n_db_models::models::asset::{object_profile, profile_primary_material};
use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_USER;
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QuerySelect};

use crate::asset::AssetDbError;
use a4n_domain::object_profile::ObjectProfileRecord;
use a4n_domain::object_profile::ObjectProfileRecordInput;

use super::read::{
    ensure_object_profile_project_exists, object_profile_by_id, object_profile_by_normalized_name,
};
use super::record::{insert_object_profile_record, replace_object_profile_record};

pub async fn create_object_profile_record(
    db: &impl crate::DbExecutor,
    project_id: i64,
    input: ObjectProfileRecordInput,
) -> Result<object_profile::Model, AssetDbError> {
    let tx = db.begin().await?;
    crate::generation::require_project_owner_lifecycle_active(&tx, project_id).await?;
    ensure_object_profile_project_exists(&tx, project_id).await?;
    if object_profile_by_normalized_name(&tx, project_id, &input.normalized_name)
        .await?
        .is_some()
    {
        return Err(AssetDbError::InvalidArgument(format!(
            "物体档案已存在: {}",
            input.name
        )));
    }
    let profile =
        insert_object_profile_record(&tx, project_id, input, PROFILE_OWNERSHIP_USER).await?;
    tx.commit().await?;
    Ok(profile)
}

pub async fn create_object_profile_read_record(
    db: &impl crate::DbExecutor,
    project_id: i64,
    input: ObjectProfileRecordInput,
) -> Result<ObjectProfileRecord, AssetDbError> {
    create_object_profile_record(db, project_id, input)
        .await
        .map(super::read::object_profile_record)
}

pub async fn update_object_profile_record(
    db: &impl crate::DbExecutor,
    object_profile_id: i64,
    expected_updated_at: &str,
    input: ObjectProfileRecordInput,
) -> Result<object_profile::Model, AssetDbError> {
    if expected_updated_at.trim().is_empty() {
        return Err(AssetDbError::InvalidArgument(
            "缺少物体档案资料版本，请刷新后重新编辑".to_string(),
        ));
    }
    let initial = object_profile_by_id(db, object_profile_id).await?;
    let tx = db.begin().await?;
    crate::generation::require_project_owner_lifecycle_active(&tx, initial.project_id).await?;
    let mut query = object_profile::Entity::find_by_id(object_profile_id);
    if tx.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let existing = query
        .one(&tx)
        .await?
        .ok_or_else(|| AssetDbError::NotFound("物体档案不存在".to_string()))?;
    if existing.project_id != initial.project_id {
        return Err(AssetDbError::InvalidArgument(format!(
            "物体档案所属项目已变化: object_profile_id={object_profile_id}"
        )));
    }
    if existing.updated_at.to_string() != expected_updated_at.trim() {
        return Err(AssetDbError::InvalidArgument(
            "物体档案已被其他操作修改，请刷新后重新编辑".to_string(),
        ));
    }
    if object_profile_by_normalized_name(&tx, existing.project_id, &input.normalized_name)
        .await?
        .as_ref()
        .is_some_and(|profile| profile.id != object_profile_id)
    {
        return Err(AssetDbError::InvalidArgument(format!(
            "物体档案已存在: {}",
            input.name
        )));
    }
    let profile = replace_object_profile_record(&tx, existing, input).await?;
    tx.commit().await?;
    Ok(profile)
}

pub async fn update_object_profile_read_record(
    db: &impl crate::DbExecutor,
    object_profile_id: i64,
    expected_updated_at: &str,
    input: ObjectProfileRecordInput,
) -> Result<ObjectProfileRecord, AssetDbError> {
    update_object_profile_record(db, object_profile_id, expected_updated_at, input)
        .await
        .map(super::read::object_profile_record)
}

pub async fn delete_object_profile_record(
    db: &impl crate::DbExecutor,
    object_profile_id: i64,
) -> Result<(), AssetDbError> {
    let initial = object_profile_by_id(db, object_profile_id).await?;
    let txn = db.begin().await?;
    crate::generation::require_project_owner_lifecycle_active(&txn, initial.project_id).await?;
    let profile = object_profile::Entity::find_by_id(object_profile_id)
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or_else(|| AssetDbError::NotFound("物体档案不存在".to_string()))?;
    if profile.project_id != initial.project_id {
        return Err(AssetDbError::InvalidArgument(format!(
            "物体档案所属项目已变化: object_profile_id={object_profile_id}"
        )));
    }
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::ProfileKind.eq("object"))
        .filter(profile_primary_material::Column::ProfileId.eq(object_profile_id))
        .exec(&txn)
        .await?;
    crate::generation::delete_generation_run_targets_for(
        &txn,
        "object_profile",
        &[object_profile_id],
    )
    .await?;
    let deleted = object_profile::Entity::delete_by_id(profile.id)
        .exec(&txn)
        .await?;
    if deleted.rows_affected == 0 {
        return Err(AssetDbError::NotFound("物体档案不存在".to_string()));
    }
    txn.commit().await?;
    Ok(())
}
