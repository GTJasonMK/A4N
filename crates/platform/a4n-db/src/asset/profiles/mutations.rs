use a4n_db_models::models::asset::{object_profile, profile_primary_material};
use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_USER;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait};

use crate::asset::AssetDbError;
use a4n_domain::object_profile::ObjectProfileRecordInput;

use super::read::{
    ensure_object_profile_project_exists, object_profile_by_id, object_profile_by_normalized_name,
};
use super::record::{insert_object_profile_record, replace_object_profile_record};

pub async fn create_object_profile_record<C>(
    db: &C,
    project_id: i64,
    input: ObjectProfileRecordInput,
) -> Result<object_profile::Model, AssetDbError>
where
    C: ConnectionTrait,
{
    ensure_object_profile_project_exists(db, project_id).await?;
    if object_profile_by_normalized_name(db, project_id, &input.normalized_name)
        .await?
        .is_some()
    {
        return Err(AssetDbError::InvalidArgument(format!(
            "物体档案已存在: {}",
            input.name
        )));
    }
    insert_object_profile_record(db, project_id, input, PROFILE_OWNERSHIP_USER).await
}

pub async fn update_object_profile_record<C>(
    db: &C,
    object_profile_id: i64,
    input: ObjectProfileRecordInput,
) -> Result<object_profile::Model, AssetDbError>
where
    C: ConnectionTrait,
{
    let existing = object_profile_by_id(db, object_profile_id).await?;
    if object_profile_by_normalized_name(db, existing.project_id, &input.normalized_name)
        .await?
        .as_ref()
        .is_some_and(|profile| profile.id != object_profile_id)
    {
        return Err(AssetDbError::InvalidArgument(format!(
            "物体档案已存在: {}",
            input.name
        )));
    }
    replace_object_profile_record(db, existing, input).await
}

pub async fn delete_object_profile_record<C>(
    db: &C,
    object_profile_id: i64,
) -> Result<(), AssetDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await?;
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
    let deleted = object_profile::Entity::delete_by_id(object_profile_id)
        .exec(&txn)
        .await?;
    if deleted.rows_affected == 0 {
        return Err(AssetDbError::NotFound("物体档案不存在".to_string()));
    }
    txn.commit().await?;
    Ok(())
}
