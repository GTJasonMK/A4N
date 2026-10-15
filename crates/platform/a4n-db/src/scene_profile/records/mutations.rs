use a4n_db_models::models::scene::scene_profile;
use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_USER;
use a4n_domain::scene_profile::{SceneProfileRecord, SceneProfileRecordInput};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, QuerySelect};

use super::{
    read::{
        ensure_project_exists, maybe_scene_profile_model_by_normalized_name,
        scene_profile_model_by_id,
    },
    record::{create_scene_profile_model, replace_scene_profile_record, scene_profile_record},
};
use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};

pub async fn create_scene_profile_record_checked(
    db: &impl crate::DbExecutor,
    project_id: i64,
    input: SceneProfileRecordInput,
) -> SceneProfileDbResult<SceneProfileRecord> {
    let tx = db.begin().await?;
    crate::generation::require_project_owner_lifecycle_active(&tx, project_id).await?;
    ensure_project_exists(&tx, project_id).await?;
    let existing =
        maybe_scene_profile_model_by_normalized_name(&tx, project_id, &input.normalized_name)
            .await?;
    if existing.is_some() {
        return Err(SceneProfileDbError::InvalidArgument(format!(
            "场景档案已存在: {}",
            input.name
        )));
    }
    let profile = create_scene_profile_model(&tx, project_id, input, PROFILE_OWNERSHIP_USER)
        .await
        .map(scene_profile_record)?;
    tx.commit().await?;
    Ok(profile)
}

pub async fn update_scene_profile_record_checked(
    db: &impl crate::DbExecutor,
    scene_profile_id: i64,
    expected_updated_at: &str,
    input: SceneProfileRecordInput,
) -> SceneProfileDbResult<SceneProfileRecord> {
    if expected_updated_at.trim().is_empty() {
        return Err(SceneProfileDbError::InvalidArgument(
            "缺少场景档案资料版本，请刷新后重新编辑".to_string(),
        ));
    }
    let initial = scene_profile_model_by_id(db, scene_profile_id).await?;
    let tx = db.begin().await?;
    crate::generation::require_project_owner_lifecycle_active(&tx, initial.project_id).await?;
    let mut query = scene_profile::Entity::find_by_id(scene_profile_id);
    if tx.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let existing = query
        .one(&tx)
        .await?
        .ok_or_else(|| SceneProfileDbError::NotFound("场景档案不存在".to_string()))?;
    if existing.project_id != initial.project_id {
        return Err(SceneProfileDbError::InvalidArgument(format!(
            "场景档案所属项目已变化: scene_profile_id={scene_profile_id}"
        )));
    }
    if existing.updated_at.to_string() != expected_updated_at.trim() {
        return Err(SceneProfileDbError::InvalidArgument(
            "场景档案已被其他操作修改，请刷新后重新编辑".to_string(),
        ));
    }
    let duplicate = maybe_scene_profile_model_by_normalized_name(
        &tx,
        existing.project_id,
        &input.normalized_name,
    )
    .await?;
    if duplicate
        .as_ref()
        .is_some_and(|profile| profile.id != scene_profile_id)
    {
        return Err(SceneProfileDbError::InvalidArgument(format!(
            "场景档案已存在: {}",
            input.name
        )));
    }
    let profile = replace_scene_profile_record(&tx, existing, input, PROFILE_OWNERSHIP_USER)
        .await
        .map(scene_profile_record)?;
    tx.commit().await?;
    Ok(profile)
}
