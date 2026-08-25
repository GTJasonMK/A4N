use a4n_core::models::ai::generated_video;
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

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
    if source_videos.is_empty() {
        return Ok(());
    }
    let mut source_video_id_case = CaseStatement::new();
    let mut source_image_id_case = CaseStatement::new();
    let mut copied_ids = Vec::with_capacity(source_videos.len());
    for video in source_videos {
        let new_id = remap_derived_required_id(&videos.ids, video.id, "generated_video.id")?;
        source_video_id_case = source_video_id_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(remap_derived_optional_id(
                &videos.ids,
                video.source_video_id,
                "generated_video.source_video_id",
            )?),
        );
        source_image_id_case = source_image_id_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(remap_derived_optional_id(
                &images.ids,
                video.source_image_id,
                "generated_video.source_image_id",
            )?),
        );
        copied_ids.push(new_id);
    }
    let updated = generated_video::Entity::update_many()
        .col_expr(
            generated_video::Column::SourceVideoId,
            source_video_id_case
                .finally(Expr::col(generated_video::Column::SourceVideoId))
                .into(),
        )
        .col_expr(
            generated_video::Column::SourceImageId,
            source_image_id_case
                .finally(Expr::col(generated_video::Column::SourceImageId))
                .into(),
        )
        .col_expr(generated_video::Column::UpdatedAt, Expr::value(now))
        .filter(generated_video::Column::Id.is_in(copied_ids))
        .exec(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if updated.rows_affected != source_videos.len() as u64 {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的生成视频来源引用数量与源记录不一致".to_string(),
            operation: "回填生成视频来源引用".to_string(),
        });
    }
    Ok(())
}
