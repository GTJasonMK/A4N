use a4n_core::models::{
    asset::object_profile, character::protagonist_profile, project::source_snapshot_profile,
    scene::scene_profile,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashSet;

use crate::generation::{GenerationDbError, validation::validate_positive_id};

use super::rules::normalize_entity_kind;

pub(crate) async fn validate_entity_refs<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
    refs: &[(&str, i64)],
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(root_project_id, "root_project_id")?;
    validate_positive_id(source_snapshot_id, "source_snapshot_id")?;
    if refs.is_empty() {
        return Ok(());
    }

    let mut normalized = Vec::with_capacity(refs.len());
    for (entity_kind, entity_id) in refs {
        validate_positive_id(*entity_id, "entity_id")?;
        let entity_kind = normalize_entity_kind(entity_kind)?;
        if entity_kind == "event" {
            return Err(GenerationDbError::InvalidArgument(
                "事件档案尚未建立持久模型，不能作为生成意图实体引用".to_string(),
            ));
        }
        if !["character", "object", "scene"].contains(&entity_kind) {
            return Err(GenerationDbError::InvalidArgument(format!(
                "实体类型无效: {entity_kind}"
            )));
        }
        normalized.push((entity_kind.to_string(), *entity_id));
    }

    let entity_ids = normalized
        .iter()
        .map(|(_, entity_id)| *entity_id)
        .collect::<HashSet<_>>();
    let snapshot_keys = source_snapshot_profile::Entity::find()
        .select_only()
        .column(source_snapshot_profile::Column::ProfileKind)
        .column(source_snapshot_profile::Column::SourceProfileId)
        .filter(source_snapshot_profile::Column::SnapshotId.eq(source_snapshot_id))
        .filter(source_snapshot_profile::Column::ProfileKind.is_in([
            "character",
            "object",
            "scene",
        ]))
        .filter(source_snapshot_profile::Column::SourceProfileId.is_in(entity_ids.iter().copied()))
        .into_tuple::<(String, i64)>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();

    let character_ids = normalized
        .iter()
        .filter(|(entity_kind, _)| entity_kind == "character")
        .map(|(_, entity_id)| *entity_id)
        .collect::<HashSet<_>>();
    let character_ids_in_root = if character_ids.is_empty() {
        HashSet::new()
    } else {
        protagonist_profile::Entity::find()
            .select_only()
            .column(protagonist_profile::Column::Id)
            .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
            .filter(protagonist_profile::Column::Id.is_in(character_ids.iter().copied()))
            .into_tuple::<i64>()
            .all(db)
            .await?
            .into_iter()
            .collect::<HashSet<_>>()
    };
    let object_ids = normalized
        .iter()
        .filter(|(entity_kind, _)| entity_kind == "object")
        .map(|(_, entity_id)| *entity_id)
        .collect::<HashSet<_>>();
    let object_ids_in_root = if object_ids.is_empty() {
        HashSet::new()
    } else {
        object_profile::Entity::find()
            .select_only()
            .column(object_profile::Column::Id)
            .filter(object_profile::Column::ProjectId.eq(root_project_id))
            .filter(object_profile::Column::Id.is_in(object_ids.iter().copied()))
            .into_tuple::<i64>()
            .all(db)
            .await?
            .into_iter()
            .collect::<HashSet<_>>()
    };
    let scene_ids = normalized
        .iter()
        .filter(|(entity_kind, _)| entity_kind == "scene")
        .map(|(_, entity_id)| *entity_id)
        .collect::<HashSet<_>>();
    let scene_ids_in_root = if scene_ids.is_empty() {
        HashSet::new()
    } else {
        scene_profile::Entity::find()
            .select_only()
            .column(scene_profile::Column::Id)
            .filter(scene_profile::Column::ProjectId.eq(root_project_id))
            .filter(scene_profile::Column::Id.is_in(scene_ids.iter().copied()))
            .into_tuple::<i64>()
            .all(db)
            .await?
            .into_iter()
            .collect::<HashSet<_>>()
    };

    for (entity_kind, entity_id) in normalized {
        if !snapshot_keys.contains(&(entity_kind.clone(), entity_id)) {
            return Err(GenerationDbError::InvalidArgument(format!(
                "源文本快照 {} 中不存在 {} 档案 {}",
                source_snapshot_id, entity_kind, entity_id
            )));
        }
        let exists_in_root = match entity_kind.as_str() {
            "character" => character_ids_in_root.contains(&entity_id),
            "object" => object_ids_in_root.contains(&entity_id),
            "scene" => scene_ids_in_root.contains(&entity_id),
            _ => false,
        };
        if !exists_in_root {
            return Err(GenerationDbError::InvalidArgument(format!(
                "实体引用 {}:{} 不属于根文本项目 {}",
                entity_kind, entity_id, root_project_id
            )));
        }
    }
    Ok(())
}
