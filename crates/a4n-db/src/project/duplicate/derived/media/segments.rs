use std::collections::HashMap;

use a4n_core::models::video_project::video_segment;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::project::duplicate::{
    ProjectDuplicateError,
    shared::{remap_derived_optional_id, remap_derived_required_id},
};

use super::super::types::DuplicatedDerivedSourceSnapshot;

pub(in crate::project::duplicate::derived) async fn copy_derived_video_segments<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    root_project_id: i64,
    duplicated_snapshot: &DuplicatedDerivedSourceSnapshot,
    spec_id_map: &HashMap<i64, i64>,
    asset_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let segments = video_segment::Entity::find()
        .filter(video_segment::Column::ProjectId.eq(source_project_id))
        .order_by_asc(video_segment::Column::OrderIndex)
        .order_by_asc(video_segment::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut segment_id_map = HashMap::new();
    for segment in &segments {
        let copied = video_segment::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(segment.source_chapter_id),
            source_snapshot_chapter_id: Set(remap_derived_optional_id(
                &duplicated_snapshot.chapter_ids,
                segment.source_snapshot_chapter_id,
                "video_segment.source_snapshot_chapter_id",
            )?),
            segment_type: Set(segment.segment_type.clone()),
            order_index: Set(segment.order_index),
            title: Set(segment.title.clone()),
            duration_secs: Set(segment.duration_secs),
            source_generation_spec_id: Set(remap_derived_optional_id(
                spec_id_map,
                segment.source_generation_spec_id,
                "video_segment.source_generation_spec_id",
            )?),
            result_media_asset_id: Set(remap_derived_optional_id(
                asset_id_map,
                segment.result_media_asset_id,
                "video_segment.result_media_asset_id",
            )?),
            previous_segment_id: Set(None),
            next_segment_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        segment_id_map.insert(segment.id, copied.id);
    }
    for segment in &segments {
        let new_id = remap_derived_required_id(&segment_id_map, segment.id, "video_segment.id")?;
        video_segment::ActiveModel {
            id: Set(new_id),
            previous_segment_id: Set(remap_derived_optional_id(
                &segment_id_map,
                segment.previous_segment_id,
                "video_segment.previous_segment_id",
            )?),
            next_segment_id: Set(remap_derived_optional_id(
                &segment_id_map,
                segment.next_segment_id,
                "video_segment.next_segment_id",
            )?),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(segment_id_map)
}
