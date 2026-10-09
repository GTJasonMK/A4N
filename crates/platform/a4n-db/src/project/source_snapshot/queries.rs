use super::{
    super::{ProjectMutationError, validate_positive_i64},
    types::{SourceSnapshotChapterIndexEntry, SourceSnapshotChapterSummary},
};
use a4n_db_models::models::project::{novel_project, source_snapshot_chapter};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

pub async fn source_snapshot_chapters(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterSummary>, ProjectMutationError> {
    let (_, snapshot_id) = source_snapshot_scope(db, project_id).await?;
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

pub async fn source_snapshot_chapter_index(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterIndexEntry>, ProjectMutationError> {
    let (root_project_id, snapshot_id) = source_snapshot_scope(db, project_id).await?;
    source_snapshot_chapter_index_for_scope(db, root_project_id, snapshot_id).await
}

/// Read a chapter index after the caller has validated the owning project and snapshot scope.
pub async fn source_snapshot_chapter_index_for_scope(
    db: &impl crate::DbExecutor,
    root_project_id: i64,
    snapshot_id: i64,
) -> Result<Vec<SourceSnapshotChapterIndexEntry>, ProjectMutationError> {
    validate_positive_i64(root_project_id, "root_project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    validate_positive_i64(snapshot_id, "snapshot_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    let rows = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Id)
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .column(source_snapshot_chapter::Column::ChapterNumber)
        .column(source_snapshot_chapter::Column::Title)
        .filter(source_snapshot_chapter::Column::RootProjectId.eq(root_project_id))
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .into_tuple::<(i64, i64, i32, Option<String>)>()
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, source_chapter_id, chapter_number, title)| SourceSnapshotChapterIndexEntry {
                id,
                snapshot_id,
                source_chapter_id,
                chapter_number,
                title,
            },
        )
        .collect())
}

pub async fn source_snapshot_chapter_for_project(
    db: &impl crate::DbExecutor,
    project_id: i64,
    snapshot_id: i64,
    source_chapter_id: i64,
) -> Result<Option<SourceSnapshotChapterSummary>, ProjectMutationError> {
    validate_positive_i64(snapshot_id, "snapshot_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    validate_positive_i64(source_chapter_id, "source_chapter_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    let (root_project_id, current_snapshot_id) = source_snapshot_scope(db, project_id).await?;
    if current_snapshot_id != snapshot_id {
        return Ok(None);
    }
    source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::RootProjectId.eq(root_project_id))
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .one(db)
        .await
        .map(|chapter| chapter.as_ref().map(source_snapshot_chapter_summary))
        .map_err(ProjectMutationError::Database)
}

pub async fn source_snapshot_chapter_content_by_snapshot_and_source(
    db: &impl crate::DbExecutor,
    root_project_id: i64,
    snapshot_id: i64,
    source_chapter_id: i64,
) -> Result<Option<String>, ProjectMutationError> {
    source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Content)
        .filter(source_snapshot_chapter::Column::RootProjectId.eq(root_project_id))
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
        ProjectSourceMode::Standalone => Err(ProjectMutationError::InvalidArgument(
            "独立漫画项目没有源文本快照章节".to_string(),
        )),
        ProjectSourceMode::Derived => source_snapshot_id.ok_or_else(|| {
            ProjectMutationError::NotFound(format!("派生项目缺少源文本快照: {}", project_id))
        }),
    }
}

async fn source_snapshot_scope(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<(i64, i64), ProjectMutationError> {
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
    let root_project_id = project.2.ok_or_else(|| {
        ProjectMutationError::InvalidArgument(format!("派生项目缺少根项目 ID: {project_id}"))
    })?;
    Ok((root_project_id, snapshot_id))
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
