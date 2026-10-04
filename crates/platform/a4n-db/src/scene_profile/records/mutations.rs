use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_USER;
use a4n_domain::scene_profile::{SceneProfileRecord, SceneProfileRecordInput};

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
    ensure_project_exists(db, project_id).await?;
    let existing =
        maybe_scene_profile_model_by_normalized_name(db, project_id, &input.normalized_name)
            .await?;
    if existing.is_some() {
        return Err(SceneProfileDbError::InvalidArgument(format!(
            "场景档案已存在: {}",
            input.name
        )));
    }
    create_scene_profile_model(db, project_id, input, PROFILE_OWNERSHIP_USER)
        .await
        .map(scene_profile_record)
}

pub async fn update_scene_profile_record_checked(
    db: &impl crate::DbExecutor,
    scene_profile_id: i64,
    input: SceneProfileRecordInput,
) -> SceneProfileDbResult<SceneProfileRecord> {
    let existing = scene_profile_model_by_id(db, scene_profile_id).await?;
    let duplicate = maybe_scene_profile_model_by_normalized_name(
        db,
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
    replace_scene_profile_record(db, existing, input, PROFILE_OWNERSHIP_USER)
        .await
        .map(scene_profile_record)
}
