use a4n_core::models::ai::generated_video;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use crate::project::duplicate::{
    ProjectDuplicateError,
    freeform::{FreeformGeneratedImageCopyRecords, FreeformGeneratedVideoCopyRecords},
    shared::{remap_derived_optional_id, remap_derived_required_id},
};

pub(super) async fn update_freeform_generated_video_source_refs_record<C>(
    db: &C,
    source_videos: &[generated_video::Model],
    images: &FreeformGeneratedImageCopyRecords,
    videos: &FreeformGeneratedVideoCopyRecords,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    for video in source_videos {
        let new_id = remap_derived_required_id(&videos.ids, video.id, "generated_video.id")?;
        generated_video::ActiveModel {
            id: Set(new_id),
            source_video_id: Set(remap_derived_optional_id(
                &videos.ids,
                video.source_video_id,
                "generated_video.source_video_id",
            )?),
            source_image_id: Set(remap_derived_optional_id(
                &images.ids,
                video.source_image_id,
                "generated_video.source_image_id",
            )?),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}
