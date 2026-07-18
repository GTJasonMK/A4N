use super::assets::copy_freeform_media_assets_record;
use super::segments::copy_freeform_video_segments_record;
use super::timelines::copy_freeform_assembly_timelines_record;
use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::derived::copy_derived_assembly_clips;
use crate::project::duplicate::freeform::{
    FreeformProjectMediaFileCopier, FreeformProjectMediaGraphCopyResult,
};
use chrono::NaiveDateTime;
use sea_orm::ConnectionTrait;
use std::collections::HashMap;

pub(in crate::project::duplicate::freeform) async fn copy_freeform_project_media_graph_record<
    C,
    Copier,
>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    generation_spec_ids: &HashMap<i64, i64>,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<FreeformProjectMediaGraphCopyResult, ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: FreeformProjectMediaFileCopier,
{
    let media_asset_ids = copy_freeform_media_assets_record(
        db,
        source_project_id,
        new_project_id,
        now,
        media_file_copier,
    )
    .await?;
    let video_segment_ids = copy_freeform_video_segments_record(
        db,
        source_project_id,
        new_project_id,
        generation_spec_ids,
        &media_asset_ids,
        now,
    )
    .await?;
    let assembly_timeline_ids = copy_freeform_assembly_timelines_record(
        db,
        source_project_id,
        new_project_id,
        &media_asset_ids,
        now,
    )
    .await?;
    let assembly_clip_ids = copy_derived_assembly_clips(
        db,
        &assembly_timeline_ids,
        &media_asset_ids,
        &video_segment_ids,
    )
    .await?;

    Ok(FreeformProjectMediaGraphCopyResult {
        media_asset_ids,
        video_segment_ids,
        assembly_timeline_ids,
        assembly_clip_ids,
    })
}
