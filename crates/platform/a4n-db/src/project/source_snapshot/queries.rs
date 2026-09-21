use super::{
    super::{ProjectMutationError, validate_positive_i64},
    types::SourceSnapshotChapterSummary,
};
use a4n_db_models::models::project::{novel_project, source_snapshot_chapter};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

pub async fn source_snapshot_chapters(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterSummary>, ProjectMutationError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    let project = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .into_tuple::<(i64, String, Option<i64>, Option<i64>)>()
        .one(db)
        .await?
        .ok_or_else(|| ProjectMutationError::NotFound(format!("项目不存在: {project_id}")))?;
    let snapshot_id = source_snapshot_id_for_identity(project.0, &project.1, project.2, project.3)?;
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

pub async fn source_snapshot_chapter_content_by_snapshot_and_source(
    db: &DatabaseConnection,
    snapshot_id: i64,
    source_chapter_id: i64,
) -> Result<Option<String>, ProjectMutationError> {
    source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Content)
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(ProjectMutationError::Database)
}

#[cfg(test)]
pub(super) fn source_snapshot_id_for_project(
    project: &novel_project::Model,
) -> Result<i64, ProjectMutationError> {
    source_snapshot_id_for_identity(
        project.id,
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
}

fn source_snapshot_id_for_identity(
    project_id: i64,
    project_kind: &str,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
) -> Result<i64, ProjectMutationError> {
    match a4n_domain::project::ProjectSourceMode::from_identity(
        project_kind,
        root_project_id,
        source_snapshot_id,
    )
    .map_err(|error| {
        ProjectMutationError::InvalidArgument(format!("项目 {}: {error}", project_id))
    })? {
        ProjectSourceMode::Self_ => Err(ProjectMutationError::InvalidArgument(
            "文本项目没有源文本快照章节".to_string(),
        )),
        ProjectSourceMode::Freeform => Err(ProjectMutationError::InvalidArgument(
            "自由视频项目没有源文本快照章节".to_string(),
        )),
        ProjectSourceMode::Derived => source_snapshot_id.ok_or_else(|| {
            ProjectMutationError::NotFound(format!("派生项目缺少源文本快照: {}", project_id))
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
