use a4n_core::models::{
    asset::object_profile, character::protagonist_profile, project::source_snapshot_profile,
    scene::scene_profile,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::generation::{GenerationDbError, validation::validate_positive_id};

use super::rules::normalize_entity_kind;

pub(crate) async fn validate_entity_ref<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
    entity_kind: &str,
    entity_id: i64,
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(root_project_id, "root_project_id")?;
    validate_positive_id(source_snapshot_id, "source_snapshot_id")?;
    validate_positive_id(entity_id, "entity_id")?;
    let entity_kind = normalize_entity_kind(entity_kind)?;
    match entity_kind {
        "character" | "object" | "scene" => {
            validate_snapshot_profile(db, source_snapshot_id, entity_kind, entity_id).await
        }
        "event" => Err(GenerationDbError::InvalidArgument(
            "事件档案尚未建立持久模型，不能作为生成意图实体引用".to_string(),
        )),
        _ => Err(GenerationDbError::InvalidArgument(format!(
            "实体类型无效: {entity_kind}"
        ))),
    }?;
    let exists_in_root = match entity_kind {
        "character" => {
            protagonist_profile::Entity::find_by_id(entity_id)
                .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
                .count(db)
                .await?
        }
        "object" => {
            object_profile::Entity::find_by_id(entity_id)
                .filter(object_profile::Column::ProjectId.eq(root_project_id))
                .count(db)
                .await?
        }
        "scene" => {
            scene_profile::Entity::find_by_id(entity_id)
                .filter(scene_profile::Column::ProjectId.eq(root_project_id))
                .count(db)
                .await?
        }
        _ => 0,
    };
    if exists_in_root == 0 {
        return Err(GenerationDbError::InvalidArgument(format!(
            "实体引用 {}:{} 不属于根文本项目 {}",
            entity_kind, entity_id, root_project_id
        )));
    }
    Ok(())
}

async fn validate_snapshot_profile<C>(
    db: &C,
    snapshot_id: i64,
    profile_kind: &str,
    profile_id: i64,
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(snapshot_id, "source_snapshot_id")?;
    validate_positive_id(profile_id, "profile_id")?;
    let profile_kind = normalize_entity_kind(profile_kind)?;
    let count = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_profile::Column::ProfileKind.eq(profile_kind))
        .filter(source_snapshot_profile::Column::SourceProfileId.eq(profile_id))
        .count(db)
        .await?;
    if count == 0 {
        return Err(GenerationDbError::InvalidArgument(format!(
            "源文本快照 {} 中不存在 {} 档案 {}",
            snapshot_id, profile_kind, profile_id
        )));
    }
    Ok(())
}
