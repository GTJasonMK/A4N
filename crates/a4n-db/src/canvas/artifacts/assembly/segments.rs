use a4n_db_models::models::{
    media::{media_asset, media_binding},
    video_project::{assembly_clip, video_segment},
};
use chrono::Utc;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;

use crate::canvas::CanvasDbError;

pub(super) async fn replace_canvas_assembly_segments<C>(
    db: &C,
    project_id: i64,
    timeline_id: i64,
    source_assets: &[media_asset::Model],
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    let stale_segment_ids = load_timeline_source_segment_ids(db, timeline_id).await?;
    assembly_clip::Entity::delete_many()
        .filter(assembly_clip::Column::TimelineId.eq(timeline_id))
        .exec(db)
        .await?;
    delete_canvas_segments(db, &stale_segment_ids).await?;

    if source_assets.is_empty() {
        return Ok(());
    }

    let now = Utc::now().naive_utc();
    let segment_rows = source_assets
        .iter()
        .enumerate()
        .map(|(index, asset)| {
            let order_index = i32::try_from(index + 1).map_err(|_| {
                CanvasDbError::InvalidArgument("画布成片片段数量超出 i32 范围".to_string())
            })?;
            Ok(video_segment::ActiveModel {
                project_id: Set(project_id),
                root_project_id: Set(asset.root_project_id),
                source_chapter_id: Set(asset.source_chapter_id),
                source_snapshot_chapter_id: Set(asset.source_snapshot_chapter_id),
                segment_type: Set("canvas_clip".to_string()),
                order_index: Set(order_index),
                title: Set(format!("画布片段 {}", index + 1)),
                duration_secs: Set(asset.duration_secs.unwrap_or(0.0).max(0.0)),
                source_generation_spec_id: Set(None),
                result_media_asset_id: Set(Some(asset.id)),
                previous_segment_id: Set(None),
                next_segment_id: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, CanvasDbError>>()?;
    let inserted_segments = video_segment::Entity::insert_many(segment_rows)
        .exec_with_returning_many(db)
        .await?;
    let segment_id_by_order = inserted_segments
        .iter()
        .map(|segment| (segment.order_index, segment.id))
        .collect::<HashMap<_, _>>();
    let segment_ids = (1..=source_assets.len())
        .map(|index| {
            let order_index = i32::try_from(index).map_err(|_| {
                CanvasDbError::InvalidArgument("画布成片片段数量超出 i32 范围".to_string())
            })?;
            segment_id_by_order
                .get(&order_index)
                .copied()
                .ok_or_else(|| {
                    CanvasDbError::InvalidArgument(format!(
                        "画布成片片段写入后缺少 order_index={order_index}"
                    ))
                })
        })
        .collect::<Result<Vec<_>, CanvasDbError>>()?;

    let mut previous_case = CaseStatement::new();
    let mut next_case = CaseStatement::new();
    let mut updated_at_case = CaseStatement::new();
    for (index, segment_id) in segment_ids.iter().copied().enumerate() {
        let previous_segment_id = index
            .checked_sub(1)
            .map(|previous_index| segment_ids[previous_index]);
        let next_segment_id = segment_ids.get(index + 1).copied();
        previous_case = previous_case.case(
            video_segment::Column::Id.eq(segment_id),
            Expr::value(previous_segment_id),
        );
        next_case = next_case.case(
            video_segment::Column::Id.eq(segment_id),
            Expr::value(next_segment_id),
        );
        if next_segment_id.is_some() {
            updated_at_case =
                updated_at_case.case(video_segment::Column::Id.eq(segment_id), Expr::value(now));
        }
    }
    if segment_ids.len() > 1 {
        video_segment::Entity::update_many()
            .col_expr(
                video_segment::Column::PreviousSegmentId,
                previous_case
                    .finally(Expr::col(video_segment::Column::PreviousSegmentId))
                    .into(),
            )
            .col_expr(
                video_segment::Column::NextSegmentId,
                next_case
                    .finally(Expr::col(video_segment::Column::NextSegmentId))
                    .into(),
            )
            .col_expr(
                video_segment::Column::UpdatedAt,
                updated_at_case
                    .finally(Expr::col(video_segment::Column::UpdatedAt))
                    .into(),
            )
            .filter(video_segment::Column::Id.is_in(segment_ids.clone()))
            .exec(db)
            .await?;
    }

    // Segment IDs are newly allocated in this transaction, so these bindings cannot pre-exist.
    let binding_rows = source_assets
        .iter()
        .enumerate()
        .map(|(index, asset)| media_binding::ActiveModel {
            media_asset_id: Set(asset.id),
            target_kind: Set("video_segment".to_string()),
            target_id: Set(segment_ids[index]),
            role: Set("generated_result".to_string()),
            created_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    media_binding::Entity::insert_many(binding_rows)
        .exec_without_returning(db)
        .await?;

    let clip_rows = source_assets
        .iter()
        .enumerate()
        .map(|(index, asset)| {
            let order_index = i32::try_from(index + 1).map_err(|_| {
                CanvasDbError::InvalidArgument("画布成片片段数量超出 i32 范围".to_string())
            })?;
            Ok(assembly_clip::ActiveModel {
                timeline_id: Set(timeline_id),
                media_asset_id: Set(asset.id),
                order_index: Set(order_index),
                trim_start_secs: Set(0.0),
                trim_end_secs: Set(asset.duration_secs.unwrap_or(0.0).max(0.0)),
                volume: Set(1.0),
                transition_kind: Set("cut".to_string()),
                source_segment_id: Set(Some(segment_ids[index])),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, CanvasDbError>>()?;
    assembly_clip::Entity::insert_many(clip_rows)
        .exec_without_returning(db)
        .await?;
    Ok(())
}

async fn load_timeline_source_segment_ids<C>(
    db: &C,
    timeline_id: i64,
) -> Result<Vec<i64>, CanvasDbError>
where
    C: ConnectionTrait,
{
    let clips = assembly_clip::Entity::find()
        .filter(assembly_clip::Column::TimelineId.eq(timeline_id))
        .all(db)
        .await?;
    Ok(clips
        .into_iter()
        .filter_map(|clip| clip.source_segment_id)
        .collect())
}

async fn delete_canvas_segments<C>(db: &C, segment_ids: &[i64]) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    if segment_ids.is_empty() {
        return Ok(());
    }
    media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq("video_segment"))
        .filter(media_binding::Column::TargetId.is_in(segment_ids.to_vec()))
        .exec(db)
        .await?;
    video_segment::Entity::delete_many()
        .filter(video_segment::Column::Id.is_in(segment_ids.to_vec()))
        .exec(db)
        .await?;
    Ok(())
}
