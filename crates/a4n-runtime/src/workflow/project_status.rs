use crate::state::AppState;
use a4n_core::error::AppError;
use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::{novel_blueprint, novel_project},
};
use a4n_core::state::{status::ProjectStatus, validator::StateValidator};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use super::chapter_cleanup::preflight_chapter_derived_cleanup;

pub async fn advance_project_status_if_allowed(
    state: &AppState,
    project_id: i64,
    target: ProjectStatus,
) -> Result<(), AppError> {
    let project = state.project.get(project_id).await?;
    let current =
        StateValidator::parse_status(&project.status).map_err(|err| AppError::InvalidState {
            current: project.status.clone(),
            operation: format!("自动推进项目状态到 {}: {err}", target.as_str()),
        })?;

    if current.ordinal() >= target.ordinal() {
        return Ok(());
    }

    state
        .project
        .forward_status(project_id, target.as_str())
        .await
        .map(|_| ())
}

pub async fn refresh_project_current_chapter(
    state: &AppState,
    project_id: i64,
) -> Result<(), AppError> {
    let max_chapter = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_desc(chapter::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .find(chapter::Model::has_available_content)
        .map_or(0, |chapter| chapter.chapter_number);

    let project = novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: project_id={project_id}")))?;

    if project.current_chapter == max_chapter {
        return Ok(());
    }

    let mut active: novel_project::ActiveModel = project.into();
    active.current_chapter = Set(max_chapter);
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active
        .update(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

fn is_character_sync_available_chapter(ch: &chapter::Model) -> bool {
    ch.has_available_content()
}

pub async fn character_sync_chapters(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<chapter::Model>, String> {
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;

    Ok(chapters
        .into_iter()
        .filter(is_character_sync_available_chapter)
        .collect())
}

pub async fn sync_project_status_from_artifacts(
    state: &AppState,
    project_id: i64,
) -> Result<(), AppError> {
    loop {
        let project = state.project.get(project_id).await?;
        let status = StateValidator::parse_status(&project.status).map_err(|err| {
            AppError::InvalidState {
                current: project.status.clone(),
                operation: format!("同步项目状态: {err}"),
            }
        })?;

        let Some(next) = next_status_from_artifacts(state, &project, status).await? else {
            break;
        };
        advance_project_status_if_allowed(state, project_id, next).await?;
    }

    sync_project_completion_status(state, project_id).await
}

async fn next_status_from_artifacts(
    state: &AppState,
    project: &novel_project::Model,
    status: ProjectStatus,
) -> Result<Option<ProjectStatus>, AppError> {
    match status {
        ProjectStatus::Draft => Ok(blueprint_total(state, project)
            .await?
            .map(|_| ProjectStatus::BlueprintReady)),
        ProjectStatus::BlueprintReady => {
            if blueprint_total(state, project).await?.is_none() {
                return Ok(None);
            }
            if any_part_outline_exists(state, project.id).await? {
                return Ok(Some(ProjectStatus::PartOutlinesReady));
            }
            Ok(any_chapter_outline_exists(state, project.id)
                .await?
                .then_some(ProjectStatus::ChapterOutlinesReady))
        }
        ProjectStatus::PartOutlinesReady => Ok(any_chapter_outline_exists(state, project.id)
            .await?
            .then_some(ProjectStatus::ChapterOutlinesReady)),
        ProjectStatus::ChapterOutlinesReady => Ok(any_chapter_has_content(state, project.id)
            .await?
            .then_some(ProjectStatus::Writing)),
        ProjectStatus::Writing | ProjectStatus::Completed => Ok(None),
    }
}

async fn blueprint_total(
    state: &AppState,
    project: &novel_project::Model,
) -> Result<Option<novel_blueprint::Model>, AppError> {
    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    Ok(blueprint)
}

async fn any_part_outline_exists(state: &AppState, project_id: i64) -> Result<bool, AppError> {
    let count = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    Ok(count > 0)
}

async fn any_chapter_outline_exists(state: &AppState, project_id: i64) -> Result<bool, AppError> {
    let count = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    Ok(count > 0)
}

async fn any_chapter_has_content(state: &AppState, project_id: i64) -> Result<bool, AppError> {
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    Ok(chapters
        .into_iter()
        .any(|chapter| chapter.has_available_content()))
}

pub async fn sync_project_completion_status(
    state: &AppState,
    project_id: i64,
) -> Result<(), AppError> {
    let _ = (state, project_id);
    Ok(())
}

pub async fn validate_chapter_generation_ready(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), String> {
    if chapter_number <= 0 {
        return Err("章节序号必须大于 0".to_string());
    }

    let has_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !has_blueprint {
        return Err("请先完成故事蓝图后再生成正文".to_string());
    }

    let has_outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !has_outline {
        return Err(format!(
            "第{chapter_number}章章节大纲未找到，请先生成该章章纲后再生成正文"
        ));
    }

    let existing = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    if existing
        .as_ref()
        .is_some_and(chapter::Model::has_available_content)
    {
        return preflight_chapter_derived_cleanup(state, project_id, chapter_number).await;
    }

    if chapter_number == 1 {
        return preflight_chapter_derived_cleanup(state, project_id, chapter_number).await;
    }

    let previous = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number - 1))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let Some(previous) = previous else {
        return Err(format!(
            "请先完成第{}章正文后，再生成第{}章",
            chapter_number - 1,
            chapter_number
        ));
    };

    if !previous.has_available_content() {
        return Err(format!(
            "第{}章尚无可用正文，请先生成或导入正文后再生成第{}章",
            chapter_number - 1,
            chapter_number
        ));
    }

    preflight_chapter_derived_cleanup(state, project_id, chapter_number).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chapter_for_sync(status: &str, content: &str) -> chapter::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter::Model {
            id: 1,
            project_id: 1,
            outline_id: None,
            chapter_number: 1,
            title: Some("第1章".to_string()),
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            status: status.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn character_sync_available_chapter_excludes_deleted_content() {
        let deleted = chapter_for_sync(chapter::STATUS_DELETED, "已删除正文");

        assert!(!is_character_sync_available_chapter(&deleted));
    }

    #[test]
    fn character_sync_available_chapter_requires_non_empty_non_deleted_content() {
        let imported = chapter_for_sync(chapter::STATUS_IMPORTED, "有效正文");
        let empty = chapter_for_sync(chapter::STATUS_IMPORTED, "  ");

        assert!(is_character_sync_available_chapter(&imported));
        assert!(!is_character_sync_available_chapter(&empty));
    }
}
