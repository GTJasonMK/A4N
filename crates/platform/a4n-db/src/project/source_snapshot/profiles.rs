use super::{
    super::ProjectMutationError,
    hashing::{hash_json, to_json},
    types::SourceSnapshotProfile,
};
use a4n_db_models::models::{
    asset::object_profile, character::protagonist_profile, project::source_snapshot_profile,
    scene::scene_profile,
};
use a4n_domain::chapter::ResolvedChapterSource;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

pub async fn scene_profiles_for_source<C: ConnectionTrait>(
    db: &C,
    source: &ResolvedChapterSource,
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
    profiles: Vec<SourceSnapshotProfile>,
) -> Result<(), ProjectMutationError>
where
    C: ConnectionTrait,
{
    let rows = profiles
        .into_iter()
        .map(|profile| source_snapshot_profile::ActiveModel {
            snapshot_id: Set(snapshot_id),
            root_project_id: Set(root_project_id),
            profile_kind: Set(profile.kind.to_string()),
            source_profile_id: Set(profile.id),
            payload_hash: Set(hash_json(&profile.payload)),
            payload_json: Set(profile.payload),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if !rows.is_empty() {
        source_snapshot_profile::Entity::insert_many(rows)
            .exec_without_returning(db)
            .await
            .map_err(ProjectMutationError::Database)?;
    }
    Ok(())
}

pub(super) async fn load_snapshot_profiles<C>(
    db: &C,
    root_project_id: i64,
) -> Result<Vec<SourceSnapshotProfile>, ProjectMutationError>
where
    C: ConnectionTrait,
{
    let mut profiles = Vec::new();
    for profile in protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        profiles.push(SourceSnapshotProfile {
            kind: "character",
            id: profile.id,
            payload: to_json(&profile)?,
        });
    }
    for profile in object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(object_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        profiles.push(SourceSnapshotProfile {
            kind: "object",
            id: profile.id,
            payload: to_json(&profile)?,
        });
    }
    for profile in scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(scene_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
    {
        profiles.push(SourceSnapshotProfile {
            kind: "scene",
            id: profile.id,
            payload: to_json(&profile)?,
        });
    }
    Ok(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db_models::models::project::project_source_snapshot;
    use sea_orm::ActiveModelTrait;
    use serde_json::json;

    #[tokio::test]
    async fn snapshot_profiles_use_the_captured_payload_after_a_live_edit() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let root = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: "冻结档案一致性".into(),
                description: String::new(),
                project_type: "novel_long".into(),
            },
        )
        .await
        .unwrap();
        let input = a4n_domain::scene_profile::normalize_scene_profile_record_input(
            serde_json::from_value(json!({
                "name": "旧车站", "scene_type": "room", "importance": "primary",
                "base_description": "旧站台", "visual_description": "挂钟与长椅",
                "spatial_layout": "狭长通道", "atmosphere": "静谧",
                "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": "旧车站",
            }))
            .unwrap(),
        )
        .unwrap();
        let profile =
            crate::scene_profile::create_scene_profile_record(&db, root.id, input, "user")
                .await
                .unwrap();
        let captured = load_snapshot_profiles(&db, root.id).await.unwrap();
        let expected_hash = hash_json(&captured[0].payload);
        let snapshot = project_source_snapshot::ActiveModel {
            root_project_id: Set(root.id),
            source_version_hash: Set("a".repeat(64)),
            chapter_scope_json: Set(json!({"start": 1, "end": 1})),
            chapter_hashes_json: Set(json!([])),
            profile_hashes_json: Set(
                json!([{"kind": "scene", "id": profile.id, "hash": expected_hash}]),
            ),
            created_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let mut changed: scene_profile::ActiveModel = profile.into();
        changed.name = Set("已经变化的车站".into());
        changed.update(&db).await.unwrap();

        freeze_profiles(&db, snapshot.id, root.id, captured)
            .await
            .unwrap();
        let frozen = source_snapshot_profile::Entity::find()
            .filter(source_snapshot_profile::Column::SnapshotId.eq(snapshot.id))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(frozen.payload_json["name"], "旧车站");
        assert_eq!(frozen.payload_hash, expected_hash);
        assert_eq!(frozen.payload_hash, hash_json(&frozen.payload_json));
    }
}
