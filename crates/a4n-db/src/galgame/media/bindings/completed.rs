use super::super::validate_completed_visual_asset;
use super::rules::validate_galgame_media_target_ids;
use crate::galgame::{
    CompletedGalgameVisualBindingRecords, GalgameMediaRecordError, require_galgame_media_text,
    validate_galgame_generation_id,
};
use a4n_core::models::{
    generation::generation_spec,
    media::{media_asset, media_binding, media_provenance},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

pub async fn completed_galgame_visual_binding_records(
    db: &DatabaseConnection,
    spec: &generation_spec::Model,
    target_entity_kind: &str,
    target_entity_id: i64,
    role: &str,
) -> Result<CompletedGalgameVisualBindingRecords, GalgameMediaRecordError> {
    validate_galgame_generation_id(spec.id, "generation_spec_id")
        .map_err(|error| GalgameMediaRecordError::InvalidArgument(error.to_string()))?;
    validate_galgame_generation_id(spec.project_id, "project_id")
        .map_err(|error| GalgameMediaRecordError::InvalidArgument(error.to_string()))?;
    validate_galgame_generation_id(spec.root_project_id, "root_project_id")
        .map_err(|error| GalgameMediaRecordError::InvalidArgument(error.to_string()))?;
    validate_galgame_media_target_ids(spec.project_id, target_entity_id)?;
    let target_entity_kind = require_galgame_media_text(target_entity_kind, "target_entity_kind")?;
    let role = require_galgame_media_text(role, "role")?;
    let Some(binding) = media_binding::Entity::find()
        .filter(media_binding::Column::TargetKind.eq(target_entity_kind))
        .filter(media_binding::Column::TargetId.eq(target_entity_id))
        .filter(media_binding::Column::Role.eq(role.to_string()))
        .order_by_desc(media_binding::Column::Id)
        .one(db)
        .await
        .map_err(GalgameMediaRecordError::Database)?
    else {
        return Err(GalgameMediaRecordError::NotFound(format!(
            "Galgame 视觉生成意图已完成但缺少媒体绑定: spec_id={}, target={}/{}",
            spec.id, target_entity_kind, target_entity_id
        )));
    };
    let Some(asset) = media_asset::Entity::find_by_id(binding.media_asset_id)
        .one(db)
        .await
        .map_err(GalgameMediaRecordError::Database)?
    else {
        return Err(GalgameMediaRecordError::NotFound(format!(
            "Galgame 视觉生成意图已完成但绑定媒体不存在: spec_id={}, binding_id={}, media_asset_id={}",
            spec.id, binding.id, binding.media_asset_id
        )));
    };
    validate_completed_visual_asset(spec, &asset)?;
    let Some(provenance) = media_provenance::Entity::find_by_id(asset.id)
        .one(db)
        .await
        .map_err(GalgameMediaRecordError::Database)?
    else {
        return Err(GalgameMediaRecordError::NotFound(format!(
            "Galgame 视觉生成意图已完成但媒体来源不存在: spec_id={}, media_asset_id={}",
            spec.id, asset.id
        )));
    };
    if provenance.generation_spec_id != Some(spec.id) {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "Galgame 视觉生成意图已完成但媒体来源未指向该 spec: spec_id={}, media_asset_id={}",
            spec.id, asset.id
        )));
    }
    Ok(CompletedGalgameVisualBindingRecords {
        binding,
        asset,
        provenance,
    })
}
