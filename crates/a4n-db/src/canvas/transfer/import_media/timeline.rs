use super::super::remap::{remap_video_studio_canvas_cache_key, remap_video_studio_required_id};
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps, VideoStudioProjectImportRows,
};
use a4n_core::models::video_project::{assembly_clip, assembly_timeline, video_segment};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(in crate::canvas::transfer) async fn import_video_studio_video_segments<C>(
    db: &C,
    project_id: i64,
    segments: &[video_segment::Model],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for segment in segments {
        let row = video_segment::ActiveModel {
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
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        maps.video_segment_ids.insert(segment.id, row.id);
    }

    for segment in segments {
        let new_id = remap_video_studio_required_id(
            &maps.video_segment_ids,
            segment.id,
            "video_segment.id",
        )?;
        video_segment::ActiveModel {
            id: Set(new_id),
            previous_segment_id: Set(segment
                .previous_segment_id
                .map(|old_id| {
                    remap_video_studio_required_id(
                        &maps.video_segment_ids,
                        old_id,
                        "video_segment.previous_segment_id",
                    )
                })
                .transpose()?),
            next_segment_id: Set(segment
                .next_segment_id
                .map(|old_id| {
                    remap_video_studio_required_id(
                        &maps.video_segment_ids,
                        old_id,
                        "video_segment.next_segment_id",
                    )
                })
                .transpose()?),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
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
    for timeline in &rows.assembly_timelines {
        let row = assembly_timeline::ActiveModel {
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
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        maps.assembly_timeline_ids.insert(timeline.id, row.id);
    }
    for clip in &rows.assembly_clips {
        let row = assembly_clip::ActiveModel {
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
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        maps.assembly_clip_ids.insert(clip.id, row.id);
    }
    Ok(())
}
