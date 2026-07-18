use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::remap::remap_freeform_metadata;
use crate::project::duplicate::freeform::{DeferredFreeformSpecTarget, FreeformCopyMaps};
use crate::project::duplicate::shared::remap_derived_required_id;
use a4n_core::models::generation::generation_spec;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use std::collections::HashMap;

pub(in crate::project::duplicate::freeform) async fn update_freeform_generation_spec_deferred_targets_record<
    C,
>(
    db: &C,
    targets: &[DeferredFreeformSpecTarget],
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    for target in targets {
        let target_entity_id = match target.target_entity_kind.as_str() {
            "canvas_node" => remap_derived_required_id(
                &maps.node_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "video_segment" => remap_derived_required_id(
                &maps.video_segment_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "assembly_timeline" => remap_derived_required_id(
                &maps.assembly_timeline_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "media_asset" => remap_derived_required_id(
                &maps.media_asset_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            other => {
                return Err(ProjectDuplicateError::InvalidArgument(format!(
                    "自由视频项目生成意图暂不支持重映射 target_entity_kind={other}"
                )));
            }
        };
        generation_spec::ActiveModel {
            id: Set(target.new_spec_id),
            target_entity_id: Set(Some(target_entity_id)),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}

pub(in crate::project::duplicate::freeform) async fn update_freeform_generation_spec_metadata_record<
    C,
>(
    db: &C,
    spec_id_map: &HashMap<i64, i64>,
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    for new_spec_id in spec_id_map.values().copied() {
        let spec = generation_spec::Entity::find_by_id(new_spec_id)
            .one(db)
            .await
            .map_err(ProjectDuplicateError::Database)?
            .ok_or_else(|| ProjectDuplicateError::InvalidState {
                current: format!("复制后的生成意图不存在: {new_spec_id}"),
                operation: "更新生成意图元数据".to_string(),
            })?;
        let params_json = remap_freeform_metadata(spec.params_json, maps)?;
        generation_spec::ActiveModel {
            id: Set(new_spec_id),
            params_json: Set(params_json),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}
