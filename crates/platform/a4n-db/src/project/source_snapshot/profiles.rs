use super::{
    super::ProjectMutationError,
    hashing::{hash_json, to_json},
};
use a4n_db_models::models::{
    asset::object_profile, character::protagonist_profile, project::source_snapshot_profile,
    scene::scene_profile,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Serialize;
use serde_json::{Value, json};

pub async fn scene_profiles_for_source<C: ConnectionTrait>(
    db: &C,
    source: &crate::project::ResolvedChapterSource,
    profile_ids: &[i64],
) -> Result<Vec<scene_profile::Model>, crate::project::ResolveChapterSourceError> {
    use crate::project::ResolveChapterSourceError;
    let ids = profile_ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if ids.iter().any(|id| *id <= 0) {
        return Err(ResolveChapterSourceError::InvalidArgument(
            "场景档案 ID 必须大于 0".into(),
        ));
    }
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let profiles = if let Some(snapshot_id) = source.source_snapshot_id {
        let rows = source_snapshot_profile::Entity::find()
            .filter(source_snapshot_profile::Column::SnapshotId.eq(snapshot_id))
            .filter(source_snapshot_profile::Column::RootProjectId.eq(source.root_project_id))
            .filter(source_snapshot_profile::Column::ProfileKind.eq("scene"))
            .filter(source_snapshot_profile::Column::SourceProfileId.is_in(ids.iter().copied()))
            .order_by_asc(source_snapshot_profile::Column::SourceProfileId)
            .all(db)
            .await?;
        rows.into_iter()
            .map(|row| {
                if hash_json(&row.payload_json) != row.payload_hash {
                    return Err(ResolveChapterSourceError::InvalidState(format!(
                        "场景快照 {} 内容哈希不一致",
                        row.id
                    )));
                }
                let profile: scene_profile::Model = serde_json::from_value(row.payload_json)
                    .map_err(|error| {
                        ResolveChapterSourceError::InvalidState(format!(
                            "场景快照 {} 无效：{error}",
                            row.id
                        ))
                    })?;
                if profile.id != row.source_profile_id
                    || profile.project_id != source.root_project_id
                {
                    return Err(ResolveChapterSourceError::InvalidState(format!(
                        "场景快照 {} 来源身份不一致",
                        row.id
                    )));
                }
                Ok(profile)
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(source.root_project_id))
            .filter(scene_profile::Column::Id.is_in(ids.iter().copied()))
            .order_by_asc(scene_profile::Column::Id)
            .all(db)
            .await?
    };
    if profiles.len() != ids.len() {
        return Err(ResolveChapterSourceError::NotFound(format!(
            "章节来源 {} 缺少引用的场景档案",
            source.source_revision_key
        )));
    }
    Ok(profiles)
}

pub(super) async fn freeze_profiles<C>(
    db: &C,
    snapshot_id: i64,
    root_project_id: i64,
) -> Result<(), ProjectMutationError>
where
    C: ConnectionTrait,
{
    let character_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
        .into_iter()
        .map(|profile| {
            snapshot_profile_active_model(
                snapshot_id,
                root_project_id,
                "character",
                profile.id,
                &profile,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !character_profiles.is_empty() {
        source_snapshot_profile::Entity::insert_many(character_profiles)
            .exec_without_returning(db)
            .await
            .map_err(ProjectMutationError::Database)?;
    }

    let object_profiles = object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
        .into_iter()
        .map(|profile| {
            snapshot_profile_active_model(
                snapshot_id,
                root_project_id,
                "object",
                profile.id,
                &profile,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !object_profiles.is_empty() {
        source_snapshot_profile::Entity::insert_many(object_profiles)
            .exec_without_returning(db)
            .await
            .map_err(ProjectMutationError::Database)?;
    }

    let scene_profiles = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
        .into_iter()
        .map(|profile| {
            snapshot_profile_active_model(
                snapshot_id,
                root_project_id,
                "scene",
                profile.id,
                &profile,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !scene_profiles.is_empty() {
        source_snapshot_profile::Entity::insert_many(scene_profiles)
            .exec_without_returning(db)
            .await
            .map_err(ProjectMutationError::Database)?;
    }
    Ok(())
}

fn snapshot_profile_active_model<T>(
    snapshot_id: i64,
    root_project_id: i64,
    profile_kind: &str,
    source_profile_id: i64,
    profile: &T,
) -> Result<source_snapshot_profile::ActiveModel, ProjectMutationError>
where
    T: Serialize,
{
    let payload = to_json(profile)?;
    Ok(source_snapshot_profile::ActiveModel {
        snapshot_id: Set(snapshot_id),
        root_project_id: Set(root_project_id),
        profile_kind: Set(profile_kind.to_string()),
        source_profile_id: Set(source_profile_id),
        payload_hash: Set(hash_json(&payload)),
        payload_json: Set(payload),
        ..Default::default()
    })
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
