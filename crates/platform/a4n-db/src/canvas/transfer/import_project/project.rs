use super::super::{SOURCE_SYNC_CURRENT, types::VideoStudioProjectImportError};
use crate::canvas::PROJECT_KIND_VIDEO;
use a4n_db_models::models::project::novel_project;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(in crate::canvas::transfer) async fn insert_video_studio_project_record<C>(
    db: &C,
    user_id: i64,
    source: &novel_project::Model,
    now: chrono::NaiveDateTime,
) -> Result<novel_project::Model, VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(source.title.clone()),
        description: Set(source.description.clone()),
        project_type: Set(source.project_type.clone()),
        project_kind: Set(PROJECT_KIND_VIDEO.to_string()),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
        status: Set(source.status.clone()),
        target_chapters: Set(source.target_chapters),
        current_chapter: Set(source.current_chapter),
        total_chapters: Set(source.total_chapters),
        chapters_per_part: Set(source.chapters_per_part),
        avatar_svg: Set(source.avatar_svg.clone()),
        avatar_animal: Set(source.avatar_animal.clone()),
        raw_blueprint: Set(source.raw_blueprint.clone()),
        import_analysis_progress: Set(source.import_analysis_progress.clone()),
        export_config: Set(source.export_config.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(VideoStudioProjectImportError::ProjectInsert)
}
