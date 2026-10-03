use super::super::remap::{remap_video_studio_canvas_cache_key, remap_video_studio_required_id};
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps, VideoStudioProjectImportRows,
};
use a4n_db_models::models::video_project::{assembly_clip, assembly_timeline, video_segment};
use a4n_domain::video_transfer::VideoSegmentRecord;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::canvas::transfer) async fn import_video_studio_video_segments<C>(
    db: &C,
    project_id: i64,
    segments: &[VideoSegmentRecord],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    if segments.is_empty() {
        return Ok(());
    }
    let inserts = segments
        .iter()
        .map(|segment| {
            Ok(video_segment::ActiveModel {
                project_id: Set(project_id),
                root_project_id: Set(project_id),
                source_chapter_id: Set(None),
                source_snapshot_chapter_id: Set(None),
                segment_type: Set(segment.segment_type.clone()),
                order_index: Set(segment.order_index),
                title: Set(segment.title.clone()),
                duration_secs: Set(segment.duration_secs),
                source_generation_spec_id: Set(segment
                    .source_generation_spec_id
                    .map(|old_id| {
                        remap_video_studio_required_id(
                            &maps.generation_spec_ids,
                            old_id,
                            "video_segment.source_generation_spec_id",
                        )
                    })
                    .transpose()?),
                result_media_asset_id: Set(segment
                    .result_media_asset_id
                    .map(|old_id| {
                        remap_video_studio_required_id(
                            &maps.media_asset_ids,
                            old_id,
                            "video_segment.result_media_asset_id",
                        )
                    })
                    .transpose()?),
                previous_segment_id: Set(None),
                next_segment_id: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, VideoStudioProjectImportError>>()?;
    let inserted = video_segment::Entity::insert_many(inserts)
        .exec_with_returning_many(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if inserted.len() != segments.len() {
        return Err(VideoStudioProjectImportError::InvalidPackage(format!(
            "视频片段批量写入返回行数不一致: inserted={}, expected={}",
            inserted.len(),
            segments.len()
        )));
    }
    for (segment, row) in segments.iter().zip(inserted) {
        maps.video_segment_ids.insert(segment.id, row.id);
    }

    let mut previous_case = CaseStatement::new();
    let mut next_case = CaseStatement::new();
    let mut segment_ids = Vec::with_capacity(segments.len());
    for segment in segments {
        let new_id = remap_video_studio_required_id(
            &maps.video_segment_ids,
            segment.id,
            "video_segment.id",
        )?;
        previous_case = previous_case.case(
            video_segment::Column::Id.eq(new_id),
            Expr::value(
                segment
                    .previous_segment_id
                    .map(|old_id| {
                        remap_video_studio_required_id(
                            &maps.video_segment_ids,
                            old_id,
                            "video_segment.previous_segment_id",
                        )
                    })
                    .transpose()?,
            ),
        );
        next_case = next_case.case(
            video_segment::Column::Id.eq(new_id),
            Expr::value(
                segment
                    .next_segment_id
                    .map(|old_id| {
                        remap_video_studio_required_id(
                            &maps.video_segment_ids,
                            old_id,
                            "video_segment.next_segment_id",
                        )
                    })
                    .transpose()?,
            ),
        );
        segment_ids.push(new_id);
    }
    let updated = video_segment::Entity::update_many()
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
        .col_expr(video_segment::Column::UpdatedAt, Expr::value(now))
        .filter(video_segment::Column::Id.is_in(segment_ids.clone()))
        .exec(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if updated.rows_affected != segment_ids.len() as u64 {
        return Err(VideoStudioProjectImportError::InvalidPackage(format!(
            "视频片段链批量回填数量不一致: updated={}, expected={}",
            updated.rows_affected,
            segment_ids.len()
        )));
    }
    Ok(())
}

pub(in crate::canvas::transfer) async fn import_video_studio_assembly_rows<C>(
    db: &C,
    project_id: i64,
    rows: &VideoStudioProjectImportRows,
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    if !rows.assembly_timelines.is_empty() {
        let inserts = rows
            .assembly_timelines
            .iter()
            .map(|timeline| {
                Ok(assembly_timeline::ActiveModel {
                    project_id: Set(project_id),
                    source_chapter_id: Set(None),
                    source_snapshot_chapter_id: Set(None),
                    version: Set(timeline.version),
                    status: Set(timeline.status.clone()),
                    preview_media_asset_id: Set(timeline
                        .preview_media_asset_id
                        .map(|old_id| {
                            remap_video_studio_required_id(
                                &maps.media_asset_ids,
                                old_id,
                                "assembly_timeline.preview_media_asset_id",
                            )
                        })
                        .transpose()?),
                    preview_cache_key: Set(remap_video_studio_canvas_cache_key(
                        timeline.preview_cache_key.as_deref(),
                        maps,
                    )?),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                })
            })
            .collect::<Result<Vec<_>, VideoStudioProjectImportError>>()?;
        let inserted = assembly_timeline::Entity::insert_many(inserts)
            .exec_with_returning_many(db)
            .await
            .map_err(VideoStudioProjectImportError::Database)?;
        if inserted.len() != rows.assembly_timelines.len() {
            return Err(VideoStudioProjectImportError::InvalidPackage(format!(
                "成片时间线批量写入返回行数不一致: inserted={}, expected={}",
                inserted.len(),
                rows.assembly_timelines.len()
            )));
        }
        for (timeline, row) in rows.assembly_timelines.iter().zip(inserted) {
            maps.assembly_timeline_ids.insert(timeline.id, row.id);
        }
    }
    if !rows.assembly_clips.is_empty() {
        let inserts = rows
            .assembly_clips
            .iter()
            .map(|clip| {
                Ok(assembly_clip::ActiveModel {
                    timeline_id: Set(remap_video_studio_required_id(
                        &maps.assembly_timeline_ids,
                        clip.timeline_id,
                        "assembly_clip.timeline_id",
                    )?),
                    media_asset_id: Set(remap_video_studio_required_id(
                        &maps.media_asset_ids,
                        clip.media_asset_id,
                        "assembly_clip.media_asset_id",
                    )?),
                    order_index: Set(clip.order_index),
                    trim_start_secs: Set(clip.trim_start_secs),
                    trim_end_secs: Set(clip.trim_end_secs),
                    volume: Set(clip.volume),
                    transition_kind: Set(clip.transition_kind.clone()),
                    source_segment_id: Set(clip
                        .source_segment_id
                        .map(|old_id| {
                            remap_video_studio_required_id(
                                &maps.video_segment_ids,
                                old_id,
                                "assembly_clip.source_segment_id",
                            )
                        })
                        .transpose()?),
                    ..Default::default()
                })
            })
            .collect::<Result<Vec<_>, VideoStudioProjectImportError>>()?;
        let inserted = assembly_clip::Entity::insert_many(inserts)
            .exec_with_returning_many(db)
            .await
            .map_err(VideoStudioProjectImportError::Database)?;
        if inserted.len() != rows.assembly_clips.len() {
            return Err(VideoStudioProjectImportError::InvalidPackage(format!(
                "成片 clip 批量写入返回行数不一致: inserted={}, expected={}",
                inserted.len(),
                rows.assembly_clips.len()
            )));
        }
        for (clip, row) in rows.assembly_clips.iter().zip(inserted) {
            maps.assembly_clip_ids.insert(clip.id, row.id);
        }
    }
    Ok(())
}
