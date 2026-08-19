use a4n_core::models::{
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor,
    },
    media::media_asset,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use crate::generation::{GenerationDbError, validation::validate_positive_id};

use super::rules::{
    normalize_entity_kind, normalize_entity_role, normalize_media_ref_role,
    normalize_text_anchor_role,
};

pub(crate) async fn insert_generation_text_anchor_ref<C>(
    db: &C,
    generation_spec_id: i64,
    text_anchor_id: i64,
    role: &str,
) -> Result<generation_spec_text_anchor::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(generation_spec_id, "generation_spec_id")?;
    validate_positive_id(text_anchor_id, "text_anchor_id")?;
    let role = normalize_text_anchor_role(role)?;
    generation_spec_text_anchor::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        text_anchor_id: Set(text_anchor_id),
        role: Set(role.to_string()),
    }
    .insert(db)
    .await
    .map_err(GenerationDbError::Database)
}

pub(crate) async fn insert_generation_entity_ref<C>(
    db: &C,
    generation_spec_id: i64,
    entity_kind: &str,
    entity_id: i64,
    role: &str,
) -> Result<generation_spec_entity_ref::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(generation_spec_id, "generation_spec_id")?;
    validate_positive_id(entity_id, "entity_id")?;
    let entity_kind = normalize_entity_kind(entity_kind)?;
    let role = normalize_entity_role(role)?;
    generation_spec_entity_ref::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        entity_kind: Set(entity_kind.to_string()),
        entity_id: Set(entity_id),
        role: Set(role.to_string()),
    }
    .insert(db)
    .await
    .map_err(GenerationDbError::Database)
}

pub(crate) async fn insert_generation_media_ref<C>(
    db: &C,
    spec: &generation_spec::Model,
    media_asset_id: i64,
    role: &str,
) -> Result<generation_spec_media_ref::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(spec.id, "generation_spec_id")?;
    validate_positive_id(media_asset_id, "media_asset_id")?;
    let role = normalize_media_ref_role(role)?;
    let asset = media_asset::Entity::find_by_id(media_asset_id)
        .one(db)
        .await?
        .ok_or_else(|| GenerationDbError::NotFound(format!("媒体资产不存在: {media_asset_id}")))?;
    if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id {
        return Err(GenerationDbError::InvalidArgument(format!(
            "生成意图 {} 不能引用跨项目媒体资产 {}",
            spec.id, asset.id
        )));
    }
    if asset.status != "completed" || asset.file_path.is_none() {
        return Err(GenerationDbError::InvalidArgument(format!(
            "媒体引用 {} 不是可用的已完成资产",
            asset.id
        )));
    }
    generation_spec_media_ref::ActiveModel {
        generation_spec_id: Set(spec.id),
        media_asset_id: Set(asset.id),
        role: Set(role.to_string()),
    }
    .insert(db)
    .await
    .map_err(GenerationDbError::Database)
}
