use super::records::{manga_prompt_active_model, manga_prompt_updated_model};
use super::validation::validate_manga_sync_drafts;
use crate::manga::MangaPipelineDbError;
use crate::manga::records::mapping::manga_page_prompt_record_from_model;
use a4n_db_models::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use a4n_domain::manga::{MangaPagePromptRecord, MangaPagePromptSyncDraft};
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;

pub async fn sync_manga_page_prompts_by_chapter(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    drafts: Vec<MangaPagePromptSyncDraft>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<MangaPagePromptRecord>, MangaPipelineDbError> {
    Ok(
        sync_manga_page_prompts_by_chapter_inner(db, project_id, chapter_id, drafts, now)
            .await?
            .into_iter()
            .map(manga_page_prompt_record_from_model)
            .collect(),
    )
}

/// Upserts one page without deleting pages that are still running.
pub async fn upsert_manga_page_prompt(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    draft: MangaPagePromptSyncDraft,
    now: chrono::NaiveDateTime,
) -> Result<MangaPagePromptRecord, MangaPipelineDbError> {
    let mut rows =
        sync_manga_page_prompts_by_chapter_inner(db, project_id, chapter_id, vec![draft], now)
            .await?;
    rows.pop()
        .map(manga_page_prompt_record_from_model)
        .ok_or_else(|| {
            MangaPipelineDbError::InvalidArgument("漫画页面 upsert 未返回记录".to_string())
        })
}

async fn sync_manga_page_prompts_by_chapter_inner(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    drafts: Vec<MangaPagePromptSyncDraft>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<chapter_manga_prompt::Model>, MangaPipelineDbError> {
    validate_manga_sync_drafts(&drafts, project_id, chapter_id)?;

    let txn = db.begin().await?;
    super::validation::validate_adopted_manga_plan(&txn, project_id, chapter_id, &drafts).await?;
    let existing = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .all(&txn)
        .await?;
    let mut existing_by_page = existing
        .into_iter()
        .map(|row| (row.page_number, row))
        .collect::<HashMap<_, _>>();

    let mut rows = Vec::with_capacity(drafts.len());
    let mut pending_inserts = Vec::new();
    let mut pending_insert_slots = Vec::new();
    let mut pending_updates = Vec::new();
    for draft in drafts {
        let result_slot = rows.len();
        if let Some(existing) = existing_by_page.remove(&draft.page_number) {
            if let Some(updated) = manga_prompt_updated_model(&existing, draft, now) {
                pending_updates.push(updated.clone());
                rows.push(Some(updated));
            } else {
                rows.push(Some(existing));
            }
        } else {
            pending_insert_slots.push((result_slot, draft.page_number));
            pending_inserts.push(manga_prompt_active_model(draft, now));
            rows.push(None);
        }
    }

    if !pending_inserts.is_empty() {
        let expected = pending_inserts.len();
        let inserted = chapter_manga_prompt::Entity::insert_many(pending_inserts)
            .exec_with_returning_many(&txn)
            .await?;
        if inserted.len() != expected {
            return Err(MangaPipelineDbError::InvalidArgument(format!(
                "漫画提示词批量插入返回行数不一致: inserted={}, expected={}",
                inserted.len(),
                expected
            )));
        }
        let inserted_by_page = inserted
            .into_iter()
            .map(|row| (row.page_number, row))
            .collect::<HashMap<_, _>>();
        for (slot, page_number) in pending_insert_slots {
            rows[slot] = inserted_by_page.get(&page_number).cloned();
        }
    }

    if !pending_updates.is_empty() {
        let ids = pending_updates.iter().map(|row| row.id).collect::<Vec<_>>();
        let mut chapter_id_case = CaseStatement::new();
        let mut project_id_case = CaseStatement::new();
        let mut page_number_case = CaseStatement::new();
        let mut image_prompt_case = CaseStatement::new();
        let mut dialogue_text_case = CaseStatement::new();
        let mut characters_in_frame_case = CaseStatement::new();
        let mut layout_case = CaseStatement::new();
        let mut generated_image_id_case = CaseStatement::new();
        let mut status_case = CaseStatement::new();
        let mut updated_at_case = CaseStatement::new();
        for row in &pending_updates {
            chapter_id_case = chapter_id_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.chapter_id),
            );
            project_id_case = project_id_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.project_id),
            );
            page_number_case = page_number_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.page_number),
            );
            image_prompt_case = image_prompt_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.image_prompt.clone()),
            );
            dialogue_text_case = dialogue_text_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.dialogue_text.clone()),
            );
            characters_in_frame_case = characters_in_frame_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.characters_in_frame.clone()).cast_as("json"),
            );
            layout_case = layout_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.layout.clone()).cast_as("json"),
            );
            generated_image_id_case = generated_image_id_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.generated_image_id),
            );
            status_case = status_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.status.clone()),
            );
            updated_at_case = updated_at_case.case(
                chapter_manga_prompt::Column::Id.eq(row.id),
                Expr::value(row.updated_at),
            );
        }
        let updated = chapter_manga_prompt::Entity::update_many()
            .col_expr(
                chapter_manga_prompt::Column::ChapterId,
                chapter_id_case
                    .finally(Expr::col(chapter_manga_prompt::Column::ChapterId))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::ProjectId,
                project_id_case
                    .finally(Expr::col(chapter_manga_prompt::Column::ProjectId))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::PageNumber,
                page_number_case
                    .finally(Expr::col(chapter_manga_prompt::Column::PageNumber))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::ImagePrompt,
                image_prompt_case
                    .finally(Expr::col(chapter_manga_prompt::Column::ImagePrompt))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::DialogueText,
                dialogue_text_case
                    .finally(Expr::col(chapter_manga_prompt::Column::DialogueText))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::CharactersInFrame,
                characters_in_frame_case
                    .finally(Expr::col(chapter_manga_prompt::Column::CharactersInFrame))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::Layout,
                layout_case
                    .finally(Expr::col(chapter_manga_prompt::Column::Layout))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::GeneratedImageId,
                generated_image_id_case
                    .finally(Expr::col(chapter_manga_prompt::Column::GeneratedImageId))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::Status,
                status_case
                    .finally(Expr::col(chapter_manga_prompt::Column::Status))
                    .into(),
            )
            .col_expr(
                chapter_manga_prompt::Column::UpdatedAt,
                updated_at_case
                    .finally(Expr::col(chapter_manga_prompt::Column::UpdatedAt))
                    .into(),
            )
            .filter(chapter_manga_prompt::Column::Id.is_in(ids.clone()))
            .exec(&txn)
            .await?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(MangaPipelineDbError::InvalidArgument(format!(
                "漫画提示词批量更新影响行数不一致: updated={}, expected={}",
                updated.rows_affected,
                ids.len()
            )));
        }
    }

    txn.commit().await?;
    rows.into_iter()
        .map(|row| {
            row.ok_or_else(|| {
                MangaPipelineDbError::InvalidArgument(
                    "漫画提示词批量插入后缺少返回记录".to_string(),
                )
            })
        })
        .collect()
}
