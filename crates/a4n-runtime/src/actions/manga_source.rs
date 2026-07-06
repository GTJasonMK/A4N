use crate::AppState;
use crate::actions::access;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_core::models::character::{character_portrait, protagonist_profile};
use a4n_core::models::project::{novel_project, source_snapshot_chapter};
use a4n_services::manga::{MangaCharacterPortraitRef, MangaReferenceContext};
use a4n_services::project::PROJECT_KIND_TEXT;
use a4n_services::project_family::load_derived_project_context;
use a4n_services::source::{ChapterSourceContext, source_metadata, source_metadata_matches};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

#[derive(Debug, Clone)]
pub(crate) struct ChapterMangaSource {
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub status: String,
    pub content: String,
}

pub(crate) fn checkpoint_matches_source(
    layout: &serde_json::Value,
    source: &ChapterMangaSource,
) -> Result<bool, AppError> {
    let current_source = source_metadata(ChapterSourceContext {
        content: &source.content,
    });
    let stored_source = layout
        .get("source")
        .ok_or_else(|| AppError::InvalidArgument("漫画流水线断点缺少 source 元数据".to_string()))?;
    Ok(source_metadata_matches(stored_source, &current_source))
}

pub(crate) fn ensure_manga_source_can_write(
    source: &ChapterMangaSource,
    operation: &str,
) -> Result<(), AppError> {
    if source.status == "deleted" {
        return Err(AppError::InvalidArgument(format!(
            "第 {} 章已删除，无法{operation}",
            source.chapter_number
        )));
    }
    if source.status == "generating" {
        return Err(AppError::InvalidArgument(format!(
            "第 {} 章正在生成中，请等待完成后再{operation}",
            source.chapter_number
        )));
    }
    Ok(())
}

pub(crate) async fn verify_manga_project_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    access::verify_project_or_derived_family_access(state, user_id, project_id).await
}

pub(crate) async fn verify_manga_project_editor(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    let project = load_manga_project(state, project_id).await?;
    if project.project_kind == PROJECT_KIND_TEXT {
        access::verify_project_editor(state, user_id, project.id).await?;
        return Ok(());
    }
    let context = load_derived_project_context(state.db.as_ref(), project.id).await?;
    access::verify_project_editor(state, user_id, context.project.id).await?;
    access::verify_project_editor(state, user_id, context.root_project.id).await
}

pub(crate) async fn collect_manga_reference_context(
    state: &AppState,
    project_id: i64,
) -> Result<MangaReferenceContext, AppError> {
    let profile_project_id = manga_profile_project_id(state, project_id).await?;
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(profile_project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    if profiles.is_empty() {
        return Ok(MangaReferenceContext::default());
    }

    let portraits = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(profile_project_id))
        .order_by_desc(character_portrait::Column::IsPrimary)
        .order_by_desc(character_portrait::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    if portraits.is_empty() {
        return Ok(MangaReferenceContext::default());
    }

    let mut character_portraits = Vec::new();
    for profile in profiles {
        let Some(portrait) = portraits
            .iter()
            .find(|portrait| portrait.profile_id == profile.id && portrait.is_primary)
            .or_else(|| {
                portraits
                    .iter()
                    .find(|portrait| portrait.profile_id == profile.id)
            })
        else {
            continue;
        };
        if portrait.image_path.trim().is_empty() {
            continue;
        }
        character_portraits.push(MangaCharacterPortraitRef {
            profile_id: profile.id,
            portrait_id: portrait.id,
            character_name: profile.name.clone(),
            image_path: portrait.image_path.clone(),
            prompt: portrait.prompt.clone(),
            portrait_type: portrait.portrait_type.clone(),
            expression: portrait.expression.clone(),
            pose: portrait.pose.clone(),
            is_primary: portrait.is_primary,
        });
    }

    Ok(MangaReferenceContext {
        character_portraits,
    })
}

pub(crate) async fn resolve_chapter_manga_source(
    state: &AppState,
    chapter_id: i64,
    project_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSource, AppError> {
    let project = load_manga_project(state, project_id).await?;
    match project.project_kind.as_str() {
        "text" => resolve_text_chapter_source(state, chapter_id, project_id, require_content).await,
        "comic" => {
            resolve_comic_snapshot_source_by_source_chapter(
                state,
                &project,
                chapter_id,
                require_content,
            )
            .await
        }
        other => Err(AppError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {other}"
        ))),
    }
}

pub(crate) async fn resolve_chapter_manga_source_by_number(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    require_content: bool,
) -> Result<ChapterMangaSource, AppError> {
    let project = load_manga_project(state, project_id).await?;
    match project.project_kind.as_str() {
        "text" => {
            let ch = chapter::Entity::find()
                .filter(chapter::Column::ProjectId.eq(project_id))
                .filter(chapter::Column::ChapterNumber.eq(chapter_number))
                .one(state.db.as_ref())
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("项目 {project_id} 第 {chapter_number} 章不存在"))
                })?;
            text_chapter_to_manga_source(ch, require_content)
        }
        "comic" => {
            resolve_comic_snapshot_source_by_number(
                state,
                &project,
                chapter_number,
                require_content,
            )
            .await
        }
        other => Err(AppError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {other}"
        ))),
    }
}

async fn resolve_text_chapter_source(
    state: &AppState,
    chapter_id: i64,
    project_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSource, AppError> {
    let ch = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("章节不存在".to_string()))?;
    if ch.project_id != project_id {
        return Err(AppError::InvalidArgument("章节不属于该项目".to_string()));
    }
    text_chapter_to_manga_source(ch, require_content)
}

fn text_chapter_to_manga_source(
    ch: chapter::Model,
    require_content: bool,
) -> Result<ChapterMangaSource, AppError> {
    if require_content && ch.content.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "章节正文为空，无法生成整页漫画提示词".to_string(),
        ));
    }

    Ok(ChapterMangaSource {
        chapter_id: ch.id,
        chapter_number: ch.chapter_number,
        status: ch.status,
        content: ch.content,
    })
}

async fn resolve_comic_snapshot_source_by_number(
    state: &AppState,
    project: &novel_project::Model,
    chapter_number: i32,
    require_content: bool,
) -> Result<ChapterMangaSource, AppError> {
    let snapshot_id = project.source_snapshot_id.ok_or_else(|| {
        AppError::InvalidArgument(format!("漫画项目缺少源文本快照: {}", project.id))
    })?;
    let chapter = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::ChapterNumber.eq(chapter_number))
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "漫画项目 {} 第 {chapter_number} 章源快照不存在",
                project.id
            ))
        })?;
    snapshot_chapter_to_manga_source(chapter, require_content)
}

async fn resolve_comic_snapshot_source_by_source_chapter(
    state: &AppState,
    project: &novel_project::Model,
    source_chapter_id: i64,
    require_content: bool,
) -> Result<ChapterMangaSource, AppError> {
    let snapshot_id = project.source_snapshot_id.ok_or_else(|| {
        AppError::InvalidArgument(format!("漫画项目缺少源文本快照: {}", project.id))
    })?;
    let chapter = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "漫画项目 {} 未包含源章节 {} 的快照",
                project.id, source_chapter_id
            ))
        })?;
    snapshot_chapter_to_manga_source(chapter, require_content)
}

fn snapshot_chapter_to_manga_source(
    chapter: source_snapshot_chapter::Model,
    require_content: bool,
) -> Result<ChapterMangaSource, AppError> {
    if require_content && chapter.content.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "源快照章节正文为空，无法生成整页漫画提示词".to_string(),
        ));
    }
    Ok(ChapterMangaSource {
        chapter_id: chapter.source_chapter_id,
        chapter_number: chapter.chapter_number,
        status: "completed".to_string(),
        content: chapter.content,
    })
}

async fn manga_profile_project_id(state: &AppState, project_id: i64) -> Result<i64, AppError> {
    let project = load_manga_project(state, project_id).await?;
    match project.project_kind.as_str() {
        "text" => Ok(project.id),
        "comic" => project.root_project_id.ok_or_else(|| {
            AppError::InvalidArgument(format!("漫画项目缺少根文本项目: {}", project.id))
        }),
        other => Err(AppError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {other}"
        ))),
    }
}

async fn load_manga_project(
    state: &AppState,
    project_id: i64,
) -> Result<novel_project::Model, AppError> {
    novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
}
