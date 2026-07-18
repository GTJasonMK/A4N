use a4n_core::models::{
    media::{media_asset, media_binding},
    video_project::{assembly_clip, video_segment},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::canvas::CanvasDbError;

use super::super::media::ensure_canvas_media_binding;

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

    let mut previous_segment_id = None;
    for (index, asset) in source_assets.iter().enumerate() {
        let order_index = i32::try_from(index + 1).map_err(|_| {
            CanvasDbError::InvalidArgument("画布成片片段数量超出 i32 范围".to_string())
        })?;
        let segment = video_segment::ActiveModel {
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
            previous_segment_id: Set(previous_segment_id),
            next_segment_id: Set(None),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        if let Some(previous_id) = previous_segment_id {
            video_segment::ActiveModel {
                id: Set(previous_id),
                next_segment_id: Set(Some(segment.id)),
                updated_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            }
            .update(db)
            .await?;
        }
        ensure_canvas_media_binding(
            db,
            asset.id,
            "video_segment",
            segment.id,
            "generated_result",
        )
        .await?;
        assembly_clip::ActiveModel {
            timeline_id: Set(timeline_id),
            media_asset_id: Set(asset.id),
            order_index: Set(order_index),
            trim_start_secs: Set(0.0),
            trim_end_secs: Set(asset.duration_secs.unwrap_or(0.0).max(0.0)),
            volume: Set(1.0),
            transition_kind: Set("cut".to_string()),
            source_segment_id: Set(Some(segment.id)),
            ..Default::default()
        }
        .insert(db)
        .await?;
        previous_segment_id = Some(segment.id);
    }
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
