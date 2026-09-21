use std::collections::HashMap;

use a4n_db_models::models::video_project::assembly_clip;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

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
    if clips.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = clips
        .iter()
        .map(|clip| {
            Ok::<_, ProjectDuplicateError>(assembly_clip::ActiveModel {
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
                transition_kind: Set(clip.transition_kind.clone()),
                source_segment_id: Set(remap_derived_optional_id(
                    segment_id_map,
                    clip.source_segment_id,
                    "assembly_clip.source_segment_id",
                )?),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    let inserted = assembly_clip::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != clips.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的成片片段数量与源记录不一致".to_string(),
            operation: "复制派生项目成片片段".to_string(),
        });
    }
    let mut clip_id_map = HashMap::with_capacity(clips.len());
    for (clip, copied) in clips.iter().zip(inserted) {
        clip_id_map.insert(clip.id, copied.id);
    }
    Ok(clip_id_map)
}
