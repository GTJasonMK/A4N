use super::{
    super::ProjectMutationError,
    hashing::{hash_json, to_json},
};
use a4n_core::models::{
    asset::object_profile, character::protagonist_profile, project::source_snapshot_profile,
    scene::scene_profile,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde::Serialize;
use serde_json::{Value, json};

pub(super) async fn freeze_profiles<C>(
    db: &C,
    snapshot_id: i64,
    root_project_id: i64,
) -> Result<(), ProjectMutationError>
where
    C: ConnectionTrait,
{
    for profile in protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        insert_snapshot_profile(
            db,
            snapshot_id,
            root_project_id,
            "character",
            profile.id,
            &profile,
        )
        .await?;
    }
    for profile in object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        insert_snapshot_profile(
            db,
            snapshot_id,
            root_project_id,
            "object",
            profile.id,
            &profile,
        )
        .await?;
    }
    for profile in scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        insert_snapshot_profile(
            db,
            snapshot_id,
            root_project_id,
            "scene",
            profile.id,
            &profile,
        )
        .await?;
    }
    Ok(())
}

async fn insert_snapshot_profile<C, T>(
    db: &C,
    snapshot_id: i64,
    root_project_id: i64,
    profile_kind: &str,
    source_profile_id: i64,
    profile: &T,
) -> Result<(), ProjectMutationError>
where
    C: ConnectionTrait,
    T: Serialize,
{
    let payload = to_json(profile)?;
    source_snapshot_profile::ActiveModel {
        snapshot_id: Set(snapshot_id),
        root_project_id: Set(root_project_id),
        profile_kind: Set(profile_kind.to_string()),
        source_profile_id: Set(source_profile_id),
        payload_hash: Set(hash_json(&payload)),
        payload_json: Set(payload),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectMutationError::Database)?;
    Ok(())
}

pub(super) async fn profile_hashes_json<C>(
    db: &C,
    root_project_id: i64,
) -> Result<Value, ProjectMutationError>
where
    C: ConnectionTrait,
{
    let mut profiles = Vec::new();
    for profile in protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        profiles.push(json!({
            "kind": "character",
            "id": profile.id,
            "hash": hash_json(&to_json(&profile)?),
        }));
    }
    for profile in object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        profiles.push(json!({
            "kind": "object",
            "id": profile.id,
            "hash": hash_json(&to_json(&profile)?),
        }));
    }
    for profile in scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        profiles.push(json!({
            "kind": "scene",
            "id": profile.id,
            "hash": hash_json(&to_json(&profile)?),
        }));
    }
    Ok(Value::Array(profiles))
}
