use a4n_core::models::scene::scene_profile;
use a4n_core::profile_ownership::PROFILE_OWNERSHIP_USER;
use sea_orm::DatabaseConnection;

use super::{
    read::{ensure_project_exists, maybe_scene_profile_by_normalized_name, scene_profile_by_id},
    record::{create_scene_profile_record, replace_scene_profile_record},
};
use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult, SceneProfileRecordInput};

pub async fn create_scene_profile_record_checked(
    db: &DatabaseConnection,
    project_id: i64,
    input: SceneProfileRecordInput,
) -> SceneProfileDbResult<scene_profile::Model> {
    ensure_project_exists(db, project_id).await?;
    let existing =
        maybe_scene_profile_by_normalized_name(db, project_id, &input.normalized_name).await?;
    if existing.is_some() {
        return Err(SceneProfileDbError::InvalidArgument(format!(
            "场景档案已存在: {}",
            input.name
        )));
    }
    create_scene_profile_record(db, project_id, input, PROFILE_OWNERSHIP_USER).await
}

pub async fn update_scene_profile_record_checked(
    db: &DatabaseConnection,
    scene_profile_id: i64,
    input: SceneProfileRecordInput,
) -> SceneProfileDbResult<scene_profile::Model> {
    let existing = scene_profile_by_id(db, scene_profile_id).await?;
    let duplicate =
        maybe_scene_profile_by_normalized_name(db, existing.project_id, &input.normalized_name)
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
    replace_scene_profile_record(db, existing, input, PROFILE_OWNERSHIP_USER).await
}
