use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::FreeformCopyMaps;
use crate::project::duplicate::freeform::remap::remap_freeform_canvas_cache_key;
use crate::project::duplicate::shared::remap_derived_optional_id;
use a4n_core::models::video_project::assembly_timeline;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::HashMap;

pub(super) async fn copy_freeform_assembly_timelines_record<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    asset_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let timelines = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(source_project_id))
        .order_by_asc(assembly_timeline::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut timeline_id_map = HashMap::new();
    for timeline in timelines {
        if timeline.source_chapter_id.is_some() || timeline.source_snapshot_chapter_id.is_some() {
            return Err(ProjectDuplicateError::InvalidState {
                current: format!("成片时间线 {} 包含派生项目上下文", timeline.id),
                operation: "复制自由视频项目成片时间线".to_string(),
            });
        }
        let copied = assembly_timeline::ActiveModel {
            project_id: Set(new_project_id),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            version: Set(timeline.version),
            status: Set(timeline.status),
            preview_media_asset_id: Set(remap_derived_optional_id(
                asset_id_map,
                timeline.preview_media_asset_id,
                "assembly_timeline.preview_media_asset_id",
            )?),
            preview_cache_key: Set(timeline.preview_cache_key),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        timeline_id_map.insert(timeline.id, copied.id);
    }
    Ok(timeline_id_map)
}

pub(in crate::project::duplicate::freeform) async fn update_freeform_assembly_timeline_cache_keys_record<
    C,
>(
    db: &C,
    timeline_id_map: &HashMap<i64, i64>,
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    for new_timeline_id in timeline_id_map.values().copied() {
        let timeline = assembly_timeline::Entity::find_by_id(new_timeline_id)
            .one(db)
            .await
            .map_err(ProjectDuplicateError::Database)?
            .ok_or_else(|| ProjectDuplicateError::InvalidState {
                current: format!("复制后的成片时间线不存在: {new_timeline_id}"),
                operation: "更新成片时间线缓存键".to_string(),
            })?;
        let preview_cache_key =
            remap_freeform_canvas_cache_key(timeline.preview_cache_key.as_deref(), maps)?;
        if preview_cache_key == timeline.preview_cache_key {
            continue;
        }
        assembly_timeline::ActiveModel {
            id: Set(new_timeline_id),
            preview_cache_key: Set(preview_cache_key),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}
