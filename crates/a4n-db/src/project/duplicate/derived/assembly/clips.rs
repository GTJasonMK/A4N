use std::collections::HashMap;

use a4n_core::models::video_project::assembly_clip;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::project::duplicate::{
    ProjectDuplicateError,
    shared::{remap_derived_optional_id, remap_derived_required_id},
};

pub(in crate::project::duplicate) async fn copy_derived_assembly_clips<C>(
    db: &C,
    timeline_id_map: &HashMap<i64, i64>,
    asset_id_map: &HashMap<i64, i64>,
    segment_id_map: &HashMap<i64, i64>,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if timeline_id_map.is_empty() {
        return Ok(HashMap::new());
    }
    let clips = assembly_clip::Entity::find()
        .filter(
            assembly_clip::Column::TimelineId
                .is_in(timeline_id_map.keys().copied().collect::<Vec<_>>()),
        )
        .order_by_asc(assembly_clip::Column::TimelineId)
        .order_by_asc(assembly_clip::Column::OrderIndex)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut clip_id_map = HashMap::new();
    for clip in clips {
        let old_id = clip.id;
        let copied = assembly_clip::ActiveModel {
            timeline_id: Set(remap_derived_required_id(
                timeline_id_map,
                clip.timeline_id,
                "assembly_clip.timeline_id",
            )?),
            media_asset_id: Set(remap_derived_required_id(
                asset_id_map,
                clip.media_asset_id,
                "assembly_clip.media_asset_id",
            )?),
            order_index: Set(clip.order_index),
            trim_start_secs: Set(clip.trim_start_secs),
            trim_end_secs: Set(clip.trim_end_secs),
            volume: Set(clip.volume),
            transition_kind: Set(clip.transition_kind),
            source_segment_id: Set(remap_derived_optional_id(
                segment_id_map,
                clip.source_segment_id,
                "assembly_clip.source_segment_id",
            )?),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        clip_id_map.insert(old_id, copied.id);
    }
    Ok(clip_id_map)
}
