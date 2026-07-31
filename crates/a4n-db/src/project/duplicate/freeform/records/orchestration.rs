use a4n_core::models::project::novel_project;
use chrono::NaiveDateTime;
use sea_orm::ConnectionTrait;

use super::{
    images::copy_freeform_generated_images_record,
    refs::update_freeform_generated_video_source_refs_record,
    videos::copy_freeform_generated_videos_record,
};
use crate::project::duplicate::{
    ProjectDuplicateError,
    freeform::{FreeformProjectMediaCopyResult, ensure_freeform_source_mode},
    shared::{ProjectCopyInsert, ProjectMediaFileCopier, insert_project_copy_record},
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
    Copier: ProjectMediaFileCopier,
{
    ensure_freeform_source_mode(&original.source_mode)?;
    let project = insert_project_copy_record(
        db,
        original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            root_project_id: None,
            parent_project_id: None,
            source_snapshot_id: None,
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint: original.raw_blueprint.clone(),
            import_analysis_progress: original.import_analysis_progress.clone(),
        },
        now,
    )
    .await?;
    let images =
        copy_freeform_generated_images_record(db, original.id, project.id, now, media_file_copier)
            .await?;
    let videos =
        copy_freeform_generated_videos_record(db, original.id, project.id, now, media_file_copier)
            .await?;
    update_freeform_generated_video_source_refs_record(db, original.id, &images, &videos, now)
        .await?;

    Ok(FreeformProjectMediaCopyResult {
        project,
        images,
        videos,
    })
}
