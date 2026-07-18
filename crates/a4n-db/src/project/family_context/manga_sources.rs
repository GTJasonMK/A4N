use super::super::{PROJECT_KIND_COMIC, PROJECT_KIND_TEXT};
use super::context::find_project_for_family;
use super::contracts::{ChapterMangaSourceSummary, ProjectFamilyDbError};
use a4n_core::models::{
    chapter::chapter,
    project::{novel_project, source_snapshot_chapter},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn chapter_manga_source_by_id(
    db: &DatabaseConnection,
    chapter_id: i64,
    project_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let project = find_project_for_family(db, project_id).await?;
    match project.project_kind.as_str() {
        PROJECT_KIND_TEXT => {
            text_chapter_manga_source_by_id(db, chapter_id, project_id, require_content).await
        }
        PROJECT_KIND_COMIC => {
            comic_snapshot_manga_source_by_source_chapter(db, &project, chapter_id, require_content)
                .await
        }
        other => Err(ProjectFamilyDbError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {other}"
        ))),
    }
}

pub async fn chapter_manga_source_by_number(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let project = find_project_for_family(db, project_id).await?;
    match project.project_kind.as_str() {
        PROJECT_KIND_TEXT => {
            let chapter = chapter::Entity::find()
                .filter(chapter::Column::ProjectId.eq(project_id))
                .filter(chapter::Column::ChapterNumber.eq(chapter_number))
                .one(db)
                .await?
                .ok_or_else(|| {
                    ProjectFamilyDbError::NotFound(format!(
                        "项目 {project_id} 第 {chapter_number} 章不存在"
                    ))
                })?;
            text_chapter_manga_source(chapter, require_content)
        }
        PROJECT_KIND_COMIC => {
            comic_snapshot_manga_source_by_number(db, &project, chapter_number, require_content)
                .await
        }
        other => Err(ProjectFamilyDbError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {other}"
        ))),
    }
}

async fn text_chapter_manga_source_by_id(
    db: &DatabaseConnection,
    chapter_id: i64,
    project_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| ProjectFamilyDbError::NotFound("章节不存在".to_string()))?;
    if chapter.project_id != project_id {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "章节不属于该项目".to_string(),
        ));
    }
    text_chapter_manga_source(chapter, require_content)
}

fn text_chapter_manga_source(
    chapter: chapter::Model,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    if require_content && chapter.content.trim().is_empty() {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "章节正文为空，无法生成整页漫画提示词".to_string(),
        ));
    }

    Ok(ChapterMangaSourceSummary {
        chapter_id: chapter.id,
        chapter_number: chapter.chapter_number,
        status: chapter.status,
        content: chapter.content,
    })
}

async fn comic_snapshot_manga_source_by_number(
    db: &DatabaseConnection,
    project: &novel_project::Model,
    chapter_number: i32,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let snapshot_id = comic_project_source_snapshot_id(project)?;
    let chapter = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "漫画项目 {} 第 {chapter_number} 章源快照不存在",
                project.id
            ))
        })?;
    snapshot_chapter_manga_source(chapter, require_content)
}

async fn comic_snapshot_manga_source_by_source_chapter(
    db: &DatabaseConnection,
    project: &novel_project::Model,
    source_chapter_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let snapshot_id = comic_project_source_snapshot_id(project)?;
    let chapter = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "漫画项目 {} 未包含源章节 {} 的快照",
                project.id, source_chapter_id
            ))
        })?;
    snapshot_chapter_manga_source(chapter, require_content)
}

fn comic_project_source_snapshot_id(
    project: &novel_project::Model,
) -> Result<i64, ProjectFamilyDbError> {
    project.source_snapshot_id.ok_or_else(|| {
        ProjectFamilyDbError::InvalidArgument(format!("漫画项目缺少源文本快照: {}", project.id))
    })
}

pub(in crate::project::family_context) fn snapshot_chapter_manga_source(
    chapter: source_snapshot_chapter::Model,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    if require_content && chapter.content.trim().is_empty() {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "源快照章节正文为空，无法生成整页漫画提示词".to_string(),
        ));
    }
    Ok(ChapterMangaSourceSummary {
        chapter_id: chapter.source_chapter_id,
        chapter_number: chapter.chapter_number,
        status: "completed".to_string(),
        content: chapter.content,
    })
}
