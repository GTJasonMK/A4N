use crate::{AppState, actions::access};
use a4n_core::error::AppError;
use a4n_core::models::{
    chapter::{
        chapter::{self, STATUS_IMPORTED},
        chapter_outline,
    },
    generation::workflow_run,
};
use a4n_services::project::PROJECT_KIND_TEXT;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct ImportChapterInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChapterContentInput {
    pub chapter_id: i64,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportedChapterSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub status: String,
    pub word_count: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterSummary {
    pub id: i64,
    pub project_id: i64,
    pub outline_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub status: String,
    pub summary: Option<String>,
    pub style_tags: Option<serde_json::Value>,
    pub analysis_data: Option<serde_json::Value>,
    pub analysis_sync_status: Option<String>,
    pub analysis_sync_can_resume: bool,
    pub analysis_sync_resume_block_reason: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

pub fn chapter_summary(ch: &chapter::Model) -> ChapterSummary {
    ChapterSummary {
        id: ch.id,
        project_id: ch.project_id,
        outline_id: ch.outline_id,
        chapter_number: ch.chapter_number,
        title: ch.title.clone(),
        content: ch.content.clone(),
        word_count: ch.word_count,
        status: ch.status.clone(),
        summary: ch.summary.clone(),
        style_tags: ch.style_tags.clone(),
        analysis_data: ch.analysis_data.clone(),
        analysis_sync_status: None,
        analysis_sync_can_resume: false,
        analysis_sync_resume_block_reason: None,
        created_at: Some(ch.created_at.to_string()),
        updated_at: Some(ch.updated_at.to_string()),
    }
}

pub async fn list_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterSummary>, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    let chapters = state.chapter.list(project_id).await?;
    let mut summaries = chapters.iter().map(chapter_summary).collect::<Vec<_>>();
    attach_chapter_analysis_sync_state(state, &chapters, &mut summaries).await?;
    Ok(summaries)
}

async fn attach_chapter_analysis_sync_state(
    state: &AppState,
    chapters: &[chapter::Model],
    summaries: &mut [ChapterSummary],
) -> Result<(), AppError> {
    if chapters.is_empty() {
        return Ok(());
    }
    let Some(project_id) = chapters.first().map(|chapter| chapter.project_id) else {
        return Ok(());
    };
    let runs = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq("chapter_analysis"))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    let mut latest_by_chapter = HashMap::<i64, workflow_run::Model>::new();
    for run in runs {
        let Some(chapter_id) = run.chapter_id else {
            continue;
        };
        latest_by_chapter.entry(chapter_id).or_insert(run);
    }
    for (chapter, summary) in chapters.iter().zip(summaries.iter_mut()) {
        let Some(run) = latest_by_chapter.get(&chapter.id) else {
            continue;
        };
        if !crate::workflow::chapter_analysis_run_input_matches(
            &run.input_fingerprint_json,
            chapter,
        ) {
            continue;
        }
        summary.analysis_sync_status = Some(run.status.clone());
        summary.analysis_sync_can_resume = chapter.analysis_data.is_some() && run.can_resume;
        summary.analysis_sync_resume_block_reason = run.resume_block_reason.clone();
    }
    Ok(())
}

pub async fn import_chapter(
    state: &AppState,
    user_id: i64,
    input: ImportChapterInput,
) -> Result<ImportedChapterSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    ensure_text_source_project(state, input.project_id, "导入章节正文").await?;
    ensure_import_chapter_number(input.chapter_number)?;
    ensure_chapter_content_not_empty(&input.content)?;

    let word_count = input.content.chars().count() as i32;
    let now = chrono::Utc::now().naive_utc();
    let normalized_title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let txn = state.db.begin().await.map_err(AppError::Database)?;
    ensure_previous_chapter_content_ready(&txn, input.project_id, input.chapter_number).await?;
    let existing = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(input.project_id))
        .filter(chapter::Column::ChapterNumber.eq(input.chapter_number))
        .one(&txn)
        .await
        .map_err(AppError::Database)?;

    if let Some(existing) = existing.as_ref() {
        ensure_chapter_not_generating(existing, "导入覆盖正文")?;
    }
    crate::workflow::preflight_chapter_content_change(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;

    let ch = if let Some(existing) = existing {
        let mut active: chapter::ActiveModel = existing.into();
        if let Some(title) = normalized_title.clone() {
            active.title = Set(Some(title));
        }
        active.content = Set(input.content.clone());
        active.word_count = Set(word_count);
        active.status = Set(STATUS_IMPORTED.to_string());
        active.summary = Set(Some(format!(
            "第{}章：{}字",
            input.chapter_number, word_count
        )));
        active.style_tags = Set(None);
        active.analysis_data = Set(None);
        active.updated_at = Set(now);
        active.update(&txn).await.map_err(AppError::Database)?
    } else {
        chapter::ActiveModel {
            project_id: Set(input.project_id),
            chapter_number: Set(input.chapter_number),
            title: Set(normalized_title.clone()),
            content: Set(input.content.clone()),
            word_count: Set(word_count),
            status: Set(STATUS_IMPORTED.to_string()),
            summary: Set(Some(format!(
                "第{}章：{}字",
                input.chapter_number, word_count
            ))),
            style_tags: Set(None),
            analysis_data: Set(None),
            outline_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(AppError::Database)?
    };

    upsert_import_outline(
        &txn,
        input.project_id,
        input.chapter_number,
        normalized_title,
        word_count,
        now,
    )
    .await?;
    txn.commit().await.map_err(AppError::Database)?;

    crate::workflow::postprocess_chapter_after_content_change(state, ch.id).await?;

    Ok(ImportedChapterSummary {
        id: ch.id,
        project_id: ch.project_id,
        chapter_number: ch.chapter_number,
        title: ch.title,
        status: STATUS_IMPORTED.to_string(),
        word_count,
    })
}

pub async fn update_chapter_content(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterContentInput,
) -> Result<ChapterSummary, AppError> {
    let chapter = state.chapter.get(input.chapter_id).await?;
    access::verify_project_editor(state, user_id, chapter.project_id).await?;
    ensure_text_source_project(state, chapter.project_id, "更新章节正文").await?;
    ensure_chapter_not_generating(&chapter, "更新章节正文")?;
    ensure_chapter_content_not_empty(&input.content)?;
    crate::workflow::preflight_chapter_content_change(
        state,
        chapter.project_id,
        chapter.chapter_number,
    )
    .await?;
    let title = input.title.as_deref();
    let updated = state
        .chapter
        .save_content_edit(input.chapter_id, &input.content, title)
        .await?;

    crate::workflow::postprocess_chapter_after_content_change(state, updated.id).await?;
    let refreshed = state.chapter.get(updated.id).await?;
    Ok(chapter_summary(&refreshed))
}

async fn upsert_import_outline<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
    normalized_title: Option<String>,
    word_count: i32,
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let outline_title = normalized_title.or_else(|| Some(format!("第{}章", chapter_number)));
    let outline_summary = format!("导入章节：{}字，摘要待生成", word_count);
    let existing_outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(AppError::Database)?;

    if let Some(outline) = existing_outline {
        let mut active: chapter_outline::ActiveModel = outline.into();
        active.title = Set(outline_title);
        active.summary = Set(Some(outline_summary));
        active.word_count_target = Set(Some(word_count));
        active.updated_at = Set(now);
        active.update(db).await.map_err(AppError::Database)?;
    } else {
        chapter_outline::ActiveModel {
            project_id: Set(project_id),
            part_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(outline_title),
            summary: Set(Some(outline_summary)),
            beats: Set(None),
            key_scenes: Set(None),
            pacing_score: Set(None),
            foreshadowing_setup: Set(None),
            foreshadowing_payoff: Set(None),
            characters_involved: Set(None),
            word_count_target: Set(Some(word_count)),
            status: Set("draft".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(())
}

fn ensure_import_chapter_number(chapter_number: i32) -> Result<(), AppError> {
    if chapter_number <= 0 {
        return Err(AppError::InvalidArgument("章节序号必须大于 0".to_string()));
    }
    Ok(())
}

fn ensure_chapter_content_not_empty(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::InvalidArgument("章节正文不能为空".to_string()));
    }
    Ok(())
}

async fn ensure_previous_chapter_content_ready<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if chapter_number <= 1 {
        return Ok(());
    }

    let previous_number = chapter_number - 1;
    let previous = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(previous_number))
        .one(db)
        .await
        .map_err(AppError::Database)?;
    let Some(previous) = previous else {
        return Err(AppError::InvalidArgument(format!(
            "请先完成第{previous_number}章正文后，再导入第{chapter_number}章"
        )));
    };
    if previous.is_generating() {
        return Err(AppError::InvalidState {
            current: previous.status,
            operation: format!("等待第{previous_number}章完成后导入第{chapter_number}章"),
        });
    }
    if !previous.has_available_content() {
        return Err(AppError::InvalidArgument(format!(
            "第{previous_number}章尚无可用正文，请先生成或导入正文后再导入第{chapter_number}章"
        )));
    }
    Ok(())
}

async fn ensure_text_source_project(
    state: &AppState,
    project_id: i64,
    operation: &str,
) -> Result<(), AppError> {
    let project = state.project.get(project_id).await?;
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(AppError::InvalidArgument(format!(
            "{operation}只能作用于文本项目"
        )));
    }
    Ok(())
}

fn ensure_chapter_not_generating(ch: &chapter::Model, operation: &str) -> Result<(), AppError> {
    if ch.is_generating() {
        return Err(AppError::InvalidState {
            current: ch.status.clone(),
            operation: format!(
                "第 {} 章正在生成中，请等待当前生成完成后再{operation}",
                ch.chapter_number
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_import_chapter_number_rejects_non_positive_number() {
        let err = ensure_import_chapter_number(0).expect_err("chapter number must be positive");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message == "章节序号必须大于 0")
        );
        assert!(ensure_import_chapter_number(1).is_ok());
    }

    #[test]
    fn ensure_chapter_content_not_empty_rejects_blank_content() {
        let err = ensure_chapter_content_not_empty(" \n\t ")
            .expect_err("blank chapter content must fail");

        assert!(matches!(err, AppError::InvalidArgument(message) if message == "章节正文不能为空"));
        assert!(ensure_chapter_content_not_empty("有效正文").is_ok());
    }
}
