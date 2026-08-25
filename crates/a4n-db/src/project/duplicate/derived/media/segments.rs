use std::collections::HashMap;

use a4n_core::models::video_project::video_segment;
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

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
    if segments.is_empty() {
        return Ok(HashMap::new());
    }
    let mut segment_id_map = HashMap::new();
    let rows = segments
        .iter()
        .map(|segment| {
            Ok::<_, ProjectDuplicateError>(video_segment::ActiveModel {
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
            })
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    let inserted = video_segment::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != segments.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的片段数量与源记录不一致".to_string(),
            operation: "复制派生项目视频片段".to_string(),
        });
    }
    for (segment, copied) in segments.iter().zip(&inserted) {
        segment_id_map.insert(segment.id, copied.id);
    }
    let mut previous_segment_id_case = CaseStatement::new();
    let mut next_segment_id_case = CaseStatement::new();
    let mut segment_ids = Vec::with_capacity(segments.len());
    for segment in &segments {
        let new_id = remap_derived_required_id(&segment_id_map, segment.id, "video_segment.id")?;
        previous_segment_id_case = previous_segment_id_case.case(
            video_segment::Column::Id.eq(new_id),
            Expr::value(remap_derived_optional_id(
                &segment_id_map,
                segment.previous_segment_id,
                "video_segment.previous_segment_id",
            )?),
        );
        next_segment_id_case = next_segment_id_case.case(
            video_segment::Column::Id.eq(new_id),
            Expr::value(remap_derived_optional_id(
                &segment_id_map,
                segment.next_segment_id,
                "video_segment.next_segment_id",
            )?),
        );
        segment_ids.push(new_id);
    }
    let updated = video_segment::Entity::update_many()
        .col_expr(
            video_segment::Column::PreviousSegmentId,
            previous_segment_id_case
                .finally(Expr::col(video_segment::Column::PreviousSegmentId))
                .into(),
        )
        .col_expr(
            video_segment::Column::NextSegmentId,
            next_segment_id_case
                .finally(Expr::col(video_segment::Column::NextSegmentId))
                .into(),
        )
        .col_expr(video_segment::Column::UpdatedAt, Expr::value(now))
        .filter(video_segment::Column::Id.is_in(segment_ids))
        .exec(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if updated.rows_affected != segments.len() as u64 {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的片段链数量与源记录不一致".to_string(),
            operation: "回填派生项目视频片段链".to_string(),
        });
    }
    Ok(segment_id_map)
}
