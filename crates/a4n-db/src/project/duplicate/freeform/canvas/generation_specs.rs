use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::{
    CopiedFreeformGenerationSpecs, DeferredFreeformSpecTarget,
};
use crate::project::duplicate::shared::remap_derived_optional_id;
use a4n_core::models::generation::generation_spec;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::HashMap;

pub(in crate::project::duplicate::freeform) async fn copy_freeform_generation_specs_record<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    text_asset_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<CopiedFreeformGenerationSpecs, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(source_project_id))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut spec_id_map = HashMap::new();
    let mut deferred_targets = Vec::new();
    for spec in &specs {
        if spec.source_snapshot_id.is_some()
            || spec.source_chapter_id.is_some()
            || spec.source_snapshot_chapter_id.is_some()
        {
            return Err(ProjectDuplicateError::InvalidState {
                current: format!("生成意图 {} 包含源快照引用", spec.id),
                operation: "复制自由视频项目生成意图".to_string(),
            });
        }
        let canvas_text_asset_id = remap_derived_optional_id(
            text_asset_id_map,
            spec.canvas_text_asset_id,
            "generation_spec.canvas_text_asset_id",
        )?;
        let target_entity_id = match (spec.target_entity_kind.as_str(), spec.target_entity_id) {
            (
                "canvas_node" | "video_segment" | "assembly_timeline" | "media_asset",
                Some(old_id),
            ) => {
                deferred_targets.push(DeferredFreeformSpecTarget {
                    new_spec_id: 0,
                    target_entity_kind: spec.target_entity_kind.clone(),
                    old_target_entity_id: old_id,
                });
                None
            }
            _ => spec.target_entity_id,
        };
        let copied = generation_spec::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(new_project_id),
            source_context_kind: Set(Some(generation_spec::SOURCE_CONTEXT_FREEFORM.to_string())),
            source_snapshot_id: Set(None),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            canvas_text_asset_id: Set(canvas_text_asset_id),
            target_kind: Set(spec.target_kind.clone()),
            target_entity_kind: Set(spec.target_entity_kind.clone()),
            target_entity_id: Set(target_entity_id),
            status: Set(spec.status.clone()),
            source_version_hash: Set(spec.source_version_hash.clone()),
            params_json: Set(spec.params_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        if let Some(target) = deferred_targets.last_mut()
            && target.new_spec_id == 0
        {
            target.new_spec_id = copied.id;
        }
        spec_id_map.insert(spec.id, copied.id);
    }
    Ok(CopiedFreeformGenerationSpecs {
        spec_ids: specs.iter().map(|spec| spec.id).collect(),
        spec_id_map,
        deferred_targets,
    })
}
