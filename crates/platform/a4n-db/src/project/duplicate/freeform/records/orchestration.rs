use a4n_db_models::models::{ai::generated_video, project::novel_project};
use a4n_domain::project::ProjectMediaFileCopier;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::{
    images::copy_freeform_generated_images_record,
    videos::{copy_freeform_generated_videos_record, freeform_generated_videos_for_copy_record},
};
use crate::project::duplicate::{
    ProjectDuplicateError,
    freeform::FreeformProjectMediaCopyResult,
    shared::{ProjectCopyInsert, insert_project_copy_record},
};

pub(in crate::project::duplicate::freeform) async fn duplicate_freeform_project_media_record<
    C,
    Copier,
>(
    db: &C,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<FreeformProjectMediaCopyResult, ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: ProjectMediaFileCopier + ?Sized,
{
    let source_videos = freeform_generated_videos_for_copy_record(db, original.id).await?;
    let project = insert_project_copy_record(
        db,
        original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            project_kind: original.project_kind.clone(),
            root_project_id: None,
            source_snapshot_id: None,
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint: original.raw_blueprint.clone(),
        },
        now,
    )
    .await?;
    let images =
        copy_freeform_generated_images_record(db, original.id, project.id, now, media_file_copier)
            .await?;
    let videos = copy_freeform_generated_videos_record(
        db,
        &source_videos,
        project.id,
        now,
        media_file_copier,
    )
    .await?;
    update_video_source_refs(db, &source_videos, &images, &videos, now).await?;

    Ok(FreeformProjectMediaCopyResult {
        project,
        images,
        videos,
    })
}

async fn update_video_source_refs<C: ConnectionTrait>(
    db: &C,
    source_videos: &[generated_video::Model],
    images: &super::super::FreeformGeneratedImageCopyRecords,
    videos: &super::super::FreeformGeneratedVideoCopyRecords,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError> {
    if source_videos.is_empty() {
        return Ok(());
    }
    let mut source_video_id_case = sea_orm::sea_query::CaseStatement::new();
    let mut source_image_id_case = sea_orm::sea_query::CaseStatement::new();
    let mut copied_ids = Vec::with_capacity(source_videos.len());
    for source in source_videos {
        let new_id = crate::project::duplicate::shared::remap_derived_required_id(
            &videos.ids,
            source.id,
            "generated_video.id",
        )?;
        source_video_id_case = source_video_id_case.case(
            generated_video::Column::Id.eq(new_id),
            sea_orm::sea_query::Expr::value(
                crate::project::duplicate::shared::remap_derived_optional_id(
                    &videos.ids,
                    source.source_video_id,
                    "generated_video.source_video_id",
                )?,
            ),
        );
        source_image_id_case = source_image_id_case.case(
            generated_video::Column::Id.eq(new_id),
            sea_orm::sea_query::Expr::value(
                crate::project::duplicate::shared::remap_derived_optional_id(
                    &images.ids,
                    source.source_image_id,
                    "generated_video.source_image_id",
                )?,
            ),
        );
        copied_ids.push(new_id);
    }
    let updated = generated_video::Entity::update_many()
        .col_expr(
            generated_video::Column::SourceVideoId,
            source_video_id_case
                .finally(sea_orm::sea_query::Expr::col(
                    generated_video::Column::SourceVideoId,
                ))
                .into(),
        )
        .col_expr(
            generated_video::Column::SourceImageId,
            source_image_id_case
                .finally(sea_orm::sea_query::Expr::col(
                    generated_video::Column::SourceImageId,
                ))
                .into(),
        )
        .col_expr(
            generated_video::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(generated_video::Column::Id.is_in(copied_ids))
        .exec(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if updated.rows_affected != source_videos.len() as u64 {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的生成视频来源引用数量与源记录不一致".to_string(),
            operation: "回填 Canvas 生成视频来源引用".to_string(),
        });
    }
    Ok(())
}
