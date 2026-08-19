use super::{
    super::{ProjectMutationError, validate_positive_i64},
    types::SourceSnapshotChapterSummary,
};
use a4n_core::models::project::{novel_project, source_snapshot_chapter};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

pub async fn source_snapshot_chapters(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterSummary>, ProjectMutationError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or_else(|| ProjectMutationError::NotFound(format!("项目不存在: {project_id}")))?;
    let snapshot_id = source_snapshot_id_for_project(&project)?;
    let chapters = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .all(db)
        .await?;
    Ok(chapters
        .iter()
        .map(source_snapshot_chapter_summary)
        .collect())
}

pub async fn source_snapshot_chapter_by_snapshot_and_source(
    db: &DatabaseConnection,
    snapshot_id: i64,
    source_chapter_id: i64,
) -> Result<Option<source_snapshot_chapter::Model>, ProjectMutationError> {
    source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .one(db)
        .await
        .map_err(ProjectMutationError::Database)
}

pub(super) fn source_snapshot_id_for_project(
    project: &novel_project::Model,
) -> Result<i64, ProjectMutationError> {
    match project.source_mode().map_err(|error| {
        ProjectMutationError::InvalidArgument(format!("项目 {}: {error}", project.id))
    })? {
        ProjectSourceMode::Self_ => Err(ProjectMutationError::InvalidArgument(
            "文本项目没有源文本快照章节".to_string(),
        )),
        ProjectSourceMode::Freeform => Err(ProjectMutationError::InvalidArgument(
            "自由视频项目没有源文本快照章节".to_string(),
        )),
        ProjectSourceMode::Derived => project.source_snapshot_id.ok_or_else(|| {
            ProjectMutationError::NotFound(format!("派生项目缺少源文本快照: {}", project.id))
        }),
    }
}

pub(super) fn source_snapshot_chapter_summary(
    chapter: &source_snapshot_chapter::Model,
) -> SourceSnapshotChapterSummary {
    SourceSnapshotChapterSummary {
        id: chapter.id,
        snapshot_id: chapter.snapshot_id,
        root_project_id: chapter.root_project_id,
        source_chapter_id: chapter.source_chapter_id,
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        summary: chapter.summary.clone(),
        content: chapter.content.clone(),
        analysis_data_json: chapter.analysis_data_json.clone(),
        content_hash: chapter.content_hash.clone(),
        analysis_hash: chapter.analysis_hash.clone(),
    }
}
