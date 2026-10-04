use super::contracts::{ChapterMangaSourceSummary, ProjectFamilyAccessRow, ProjectFamilyDbError};
use super::project_family_access_row;
use a4n_db_models::models::{
    chapter::chapter,
    project::{source_snapshot_chapter, standalone_comic_text_source},
};
use a4n_domain::project::{ProjectKind, ProjectSourceMode};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashMap;

pub async fn chapter_manga_source_by_id(
    db: &impl crate::DbExecutor,
    chapter_id: i64,
    project_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let project = project_family_access_row(db, project_id).await?;
    match ProjectKind::parse(&project.project_kind) {
        Some(ProjectKind::Text) => {
            text_chapter_manga_source_by_id(db, chapter_id, project_id, require_content).await
        }
        Some(ProjectKind::Comic) => match comic_project_source_mode(&project)? {
            ProjectSourceMode::Derived => {
                comic_snapshot_manga_source_by_source_chapter(
                    db,
                    &project,
                    chapter_id,
                    require_content,
                )
                .await
            }
            ProjectSourceMode::Standalone => {
                standalone_comic_manga_source_by_id(db, &project, chapter_id, require_content).await
            }
            _ => Err(invalid_comic_mode(project_id)),
        },
        _ => Err(ProjectFamilyDbError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {}",
            project.project_kind
        ))),
    }
}

pub async fn chapter_manga_source_by_number(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_number: i32,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let project = project_family_access_row(db, project_id).await?;
    match ProjectKind::parse(&project.project_kind) {
        Some(ProjectKind::Text) => {
            let chapter = chapter::Entity::find_current()
                .select_only()
                .column(chapter::Column::Id)
                .column(chapter::Column::ChapterNumber)
                .column(chapter::Column::Content)
                .column(chapter::Column::Status)
                .filter(chapter::Column::ProjectId.eq(project_id))
                .filter(chapter::Column::ChapterNumber.eq(chapter_number))
                .into_tuple::<(i64, i32, String, String)>()
                .one(db)
                .await?
                .ok_or_else(|| {
                    ProjectFamilyDbError::NotFound(format!(
                        "项目 {project_id} 第 {chapter_number} 章不存在"
                    ))
                })?;
            text_chapter_manga_source_fields(
                chapter.0,
                chapter.1,
                chapter.2,
                chapter.3,
                require_content,
            )
        }
        Some(ProjectKind::Comic) => match comic_project_source_mode(&project)? {
            ProjectSourceMode::Derived => {
                comic_snapshot_manga_source_by_number(db, &project, chapter_number, require_content)
                    .await
            }
            ProjectSourceMode::Standalone => {
                standalone_comic_manga_source_by_number(
                    db,
                    &project,
                    chapter_number,
                    require_content,
                )
                .await
            }
            _ => Err(invalid_comic_mode(project_id)),
        },
        _ => Err(ProjectFamilyDbError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {}",
            project.project_kind
        ))),
    }
}

pub async fn chapter_manga_sources_by_ids(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_ids: &[i64],
    require_content: bool,
) -> Result<Vec<ChapterMangaSourceSummary>, ProjectFamilyDbError> {
    if project_id <= 0 {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "project_id 必须是正整数".to_string(),
        ));
    }
    for chapter_id in chapter_ids {
        if *chapter_id <= 0 {
            return Err(ProjectFamilyDbError::InvalidArgument(
                "chapter_id 必须是正整数".to_string(),
            ));
        }
    }
    if chapter_ids.is_empty() {
        return Ok(Vec::new());
    }
    let project = project_family_access_row(db, project_id).await?;
    let mut sources = HashMap::with_capacity(chapter_ids.len());
    match ProjectKind::parse(&project.project_kind) {
        Some(ProjectKind::Text) => {
            let chapters = chapter::Entity::find_current()
                .select_only()
                .column(chapter::Column::Id)
                .column(chapter::Column::ChapterNumber)
                .column(chapter::Column::Content)
                .column(chapter::Column::Status)
                .filter(chapter::Column::ProjectId.eq(project_id))
                .filter(chapter::Column::Id.is_in(chapter_ids.iter().copied()))
                .into_tuple::<(i64, i32, String, String)>()
                .all(db)
                .await?;
            for (id, chapter_number, content, status) in chapters {
                sources.insert(
                    id,
                    text_chapter_manga_source_fields(
                        id,
                        chapter_number,
                        content,
                        status,
                        require_content,
                    )?,
                );
            }
        }
        Some(ProjectKind::Comic) => match comic_project_source_mode(&project)? {
            ProjectSourceMode::Derived => {
                let snapshot_id = comic_project_source_snapshot_id(&project)?;
                let chapters = source_snapshot_chapter::Entity::find()
                    .select_only()
                    .column(source_snapshot_chapter::Column::SourceChapterId)
                    .column(source_snapshot_chapter::Column::ChapterNumber)
                    .column(source_snapshot_chapter::Column::Content)
                    .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
                    .filter(
                        source_snapshot_chapter::Column::SourceChapterId
                            .is_in(chapter_ids.iter().copied()),
                    )
                    .into_tuple::<(i64, i32, String)>()
                    .all(db)
                    .await?;
                for (chapter_id, chapter_number, content) in chapters {
                    let source = snapshot_chapter_manga_source_fields(
                        chapter_id,
                        chapter_number,
                        content,
                        require_content,
                    )?;
                    sources.insert(source.chapter_id, source);
                }
            }
            ProjectSourceMode::Standalone => {
                let chapters = chapter::Entity::find_current()
                    .filter(chapter::Column::ProjectId.eq(project_id))
                    .filter(chapter::Column::Id.is_in(chapter_ids.iter().copied()))
                    .all(db)
                    .await?;
                let source_chapter_ids = standalone_comic_text_source::Entity::find()
                    .select_only()
                    .column(standalone_comic_text_source::Column::ChapterId)
                    .filter(standalone_comic_text_source::Column::ProjectId.eq(project_id))
                    .filter(
                        standalone_comic_text_source::Column::ChapterId
                            .is_in(chapter_ids.iter().copied()),
                    )
                    .into_tuple::<i64>()
                    .all(db)
                    .await?
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>();
                for chapter in chapters {
                    if !source_chapter_ids.contains(&chapter.id) {
                        continue;
                    }
                    let source = standalone_comic_manga_source_fields(
                        chapter.id,
                        chapter.chapter_number,
                        chapter.content,
                        chapter.status,
                        require_content,
                    )?;
                    sources.insert(source.chapter_id, source);
                }
            }
            _ => return Err(invalid_comic_mode(project_id)),
        },
        _ => {
            return Err(ProjectFamilyDbError::InvalidArgument(format!(
                "项目 {project_id} 不是文本项目或漫画项目: {}",
                project.project_kind
            )));
        }
    }

    chapter_ids
        .iter()
        .map(|chapter_id| {
            sources
                .remove(chapter_id)
                .ok_or_else(|| ProjectFamilyDbError::NotFound(format!("章节不存在: {chapter_id}")))
        })
        .collect()
}

async fn text_chapter_manga_source_by_id(
    db: &impl crate::DbExecutor,
    chapter_id: i64,
    project_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::ProjectId)
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Content)
        .column(chapter::Column::Status)
        .into_tuple::<(i64, i64, i32, String, String)>()
        .one(db)
        .await?
        .ok_or_else(|| ProjectFamilyDbError::NotFound("章节不存在".to_string()))?;
    if chapter.1 != project_id {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "章节不属于该项目".to_string(),
        ));
    }
    text_chapter_manga_source_fields(chapter.0, chapter.2, chapter.3, chapter.4, require_content)
}

fn text_chapter_manga_source_fields(
    chapter_id: i64,
    chapter_number: i32,
    content: String,
    status: String,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    if require_content && content.trim().is_empty() {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "章节正文为空，无法生成整页漫画提示词".to_string(),
        ));
    }

    Ok(ChapterMangaSourceSummary {
        chapter_id,
        chapter_number,
        status,
        content,
    })
}

async fn comic_snapshot_manga_source_by_number(
    db: &impl crate::DbExecutor,
    project: &ProjectFamilyAccessRow,
    chapter_number: i32,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let snapshot_id = comic_project_source_snapshot_id(project)?;
    let chapter = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .column(source_snapshot_chapter::Column::ChapterNumber)
        .column(source_snapshot_chapter::Column::Content)
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::ChapterNumber.eq(chapter_number))
        .into_tuple::<(i64, i32, String)>()
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "漫画项目 {} 第 {chapter_number} 章源快照不存在",
                project.id
            ))
        })?;
    snapshot_chapter_manga_source_fields(chapter.0, chapter.1, chapter.2, require_content)
}

async fn comic_snapshot_manga_source_by_source_chapter(
    db: &impl crate::DbExecutor,
    project: &ProjectFamilyAccessRow,
    source_chapter_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let snapshot_id = comic_project_source_snapshot_id(project)?;
    let chapter = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .column(source_snapshot_chapter::Column::ChapterNumber)
        .column(source_snapshot_chapter::Column::Content)
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .into_tuple::<(i64, i32, String)>()
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "漫画项目 {} 未包含源章节 {} 的快照",
                project.id, source_chapter_id
            ))
        })?;
    snapshot_chapter_manga_source_fields(chapter.0, chapter.1, chapter.2, require_content)
}

fn comic_project_source_snapshot_id(
    project: &ProjectFamilyAccessRow,
) -> Result<i64, ProjectFamilyDbError> {
    project.source_snapshot_id.ok_or_else(|| {
        ProjectFamilyDbError::InvalidArgument(format!("漫画项目缺少源文本快照: {}", project.id))
    })
}

fn comic_project_source_mode(
    project: &ProjectFamilyAccessRow,
) -> Result<ProjectSourceMode, ProjectFamilyDbError> {
    ProjectSourceMode::from_identity(
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
    .map_err(|error| ProjectFamilyDbError::InvalidArgument(format!("项目 {}: {error}", project.id)))
}

fn invalid_comic_mode(project_id: i64) -> ProjectFamilyDbError {
    ProjectFamilyDbError::InvalidArgument(format!("漫画项目 {project_id} 的来源模式无效"))
}

async fn standalone_comic_manga_source_by_number(
    db: &impl crate::DbExecutor,
    project: &ProjectFamilyAccessRow,
    chapter_number: i32,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let chapter = chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project.id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "独立漫画项目 {} 第 {chapter_number} 个文本来源不存在",
                project.id
            ))
        })?;
    standalone_comic_manga_source_by_id(db, project, chapter.id, require_content).await
}

async fn standalone_comic_manga_source_by_id(
    db: &impl crate::DbExecutor,
    project: &ProjectFamilyAccessRow,
    chapter_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    let chapter = chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project.id))
        .filter(chapter::Column::Id.eq(chapter_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "独立漫画项目 {} 文本来源章节不存在: {chapter_id}",
                project.id
            ))
        })?;
    let source = standalone_comic_text_source::Entity::find()
        .filter(standalone_comic_text_source::Column::ProjectId.eq(project.id))
        .filter(standalone_comic_text_source::Column::ChapterId.eq(chapter_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "独立漫画项目 {} 未包含来源章节 {chapter_id}",
                project.id
            ))
        })?;
    if a4n_domain::source_text_hash(&chapter.content) != source.content_hash {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
            "独立漫画来源章节 {chapter_id} 内容哈希不匹配"
        )));
    }
    standalone_comic_manga_source_fields(
        chapter.id,
        chapter.chapter_number,
        chapter.content,
        chapter.status,
        require_content,
    )
}

fn standalone_comic_manga_source_fields(
    chapter_id: i64,
    chapter_number: i32,
    content: String,
    status: String,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    if require_content && content.trim().is_empty() {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "独立漫画文本来源为空，无法生成整页漫画提示词".to_string(),
        ));
    }
    Ok(ChapterMangaSourceSummary {
        chapter_id,
        chapter_number,
        status,
        content,
    })
}

#[cfg(test)]
pub(super) fn snapshot_chapter_manga_source(
    chapter: source_snapshot_chapter::Model,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    snapshot_chapter_manga_source_fields(
        chapter.source_chapter_id,
        chapter.chapter_number,
        chapter.content,
        require_content,
    )
}

fn snapshot_chapter_manga_source_fields(
    chapter_id: i64,
    chapter_number: i32,
    content: String,
    require_content: bool,
) -> Result<ChapterMangaSourceSummary, ProjectFamilyDbError> {
    if require_content && content.trim().is_empty() {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "源快照章节正文为空，无法生成整页漫画提示词".to_string(),
        ));
    }
    Ok(ChapterMangaSourceSummary {
        chapter_id,
        chapter_number,
        status: "completed".to_string(),
        content,
    })
}
