use a4n_db_models::models::chapter::chapter_outline;
use a4n_domain::chapter::ChapterOutlineRecord;
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait,
};

use super::super::record_mapping::chapter_outline_record_from_model;
use super::super::types::OutlineDbError;
use super::records::chapter_outline_active_model;
use super::rules::index_chapter_outlines_by_number;
use a4n_domain::chapter::ChapterOutlineSyncDraft;

/// 按 chapter_number 增量同步章节大纲（更新已有、插入缺失）。
pub async fn sync_chapter_outline_records_by_number_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    part_id: Option<i64>,
    drafts: Vec<ChapterOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<ChapterOutlineRecord>, OutlineDbError> {
    let txn = db.begin().await?;
    super::super::lock_outline_project_for_write(&txn, project_id, "同步章节大纲").await?;
    let outlines =
        sync_chapter_outline_records_by_number(&txn, project_id, part_id, drafts, now).await?;
    txn.commit().await?;
    Ok(outlines)
}

/// 按 chapter_number 增量同步章节大纲（更新已有、插入缺失）。
///
/// 调用方已经持有事务时使用该入口；普通业务写入应使用
/// [`sync_chapter_outline_records_by_number_transaction`]。
pub async fn sync_chapter_outline_records_by_number<C>(
    db: &C,
    project_id: i64,
    part_id: Option<i64>,
    drafts: Vec<ChapterOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<ChapterOutlineRecord>, OutlineDbError>
where
    C: ConnectionTrait,
{
    if drafts.is_empty() {
        return Ok(Vec::new());
    }

    let chapter_numbers = drafts
        .iter()
        .map(|draft| draft.chapter_number)
        .collect::<Vec<_>>();

    let existing = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.is_in(chapter_numbers))
        .all(db)
        .await?;
    let mut existing_by_number = index_chapter_outlines_by_number(existing)?;

    let mut results = Vec::with_capacity(drafts.len());
    let mut pending_inserts = Vec::new();
    let mut pending_insert_slots = Vec::new();
    let mut pending_updates = Vec::new();
    for draft in drafts {
        let result_slot = results.len();
        if let Some(existing) = existing_by_number.remove(&draft.chapter_number) {
            let updated_at =
                std::cmp::max(now, existing.updated_at + chrono::Duration::microseconds(1));
            let mut updated = existing;
            updated.part_id = part_id;
            updated.title = Some(draft.title);
            updated.summary = Some(draft.summary);
            updated.beats = draft.beats;
            updated.key_scenes = draft.key_scenes;
            updated.pacing_score = draft.pacing_score;
            updated.foreshadowing_setup = draft.foreshadowing_setup;
            updated.foreshadowing_payoff = draft.foreshadowing_payoff;
            updated.characters_involved = draft.characters_involved;
            updated.word_count_target = draft.word_count_target;
            updated.status = "draft".to_string();
            updated.updated_at = updated_at;
            pending_updates.push(updated.clone());
            results.push(Some(updated));
        } else {
            pending_inserts.push(chapter_outline_active_model(
                project_id, part_id, &draft, now,
            ));
            pending_insert_slots.push((result_slot, draft.chapter_number));
            results.push(None);
        }
    }

    if !pending_inserts.is_empty() {
        let expected = pending_inserts.len();
        let inserted = chapter_outline::Entity::insert_many(pending_inserts)
            .exec_with_returning_many(db)
            .await?;
        if inserted.len() != expected {
            return Err(OutlineDbError::InvalidArgument(format!(
                "章节大纲批量插入返回行数不一致: inserted={}, expected={}",
                inserted.len(),
                expected
            )));
        }
        let inserted_by_number = inserted
            .into_iter()
            .map(|outline| (outline.chapter_number, outline))
            .collect::<std::collections::BTreeMap<_, _>>();
        for (slot, chapter_number) in pending_insert_slots {
            results[slot] = inserted_by_number.get(&chapter_number).cloned();
        }
    }

    if !pending_updates.is_empty() {
        let update_ids = pending_updates
            .iter()
            .map(|outline| outline.id)
            .collect::<Vec<_>>();
        crate::project::clear_chapter_outline_provenance_many(db, &update_ids).await?;

        let mut part_id_case = CaseStatement::new();
        let mut title_case = CaseStatement::new();
        let mut summary_case = CaseStatement::new();
        let mut beats_case = CaseStatement::new();
        let mut key_scenes_case = CaseStatement::new();
        let mut pacing_score_case = CaseStatement::new();
        let mut foreshadowing_setup_case = CaseStatement::new();
        let mut foreshadowing_payoff_case = CaseStatement::new();
        let mut characters_involved_case = CaseStatement::new();
        let mut word_count_target_case = CaseStatement::new();
        let mut status_case = CaseStatement::new();
        let mut updated_at_case = CaseStatement::new();
        for outline in &pending_updates {
            part_id_case = part_id_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.part_id),
            );
            title_case = title_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.title.clone()),
            );
            summary_case = summary_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.summary.clone()),
            );
            beats_case = beats_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.beats.clone()).cast_as("json"),
            );
            key_scenes_case = key_scenes_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.key_scenes.clone()).cast_as("json"),
            );
            pacing_score_case = pacing_score_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.pacing_score),
            );
            foreshadowing_setup_case = foreshadowing_setup_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.foreshadowing_setup.clone()).cast_as("json"),
            );
            foreshadowing_payoff_case = foreshadowing_payoff_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.foreshadowing_payoff.clone()).cast_as("json"),
            );
            characters_involved_case = characters_involved_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.characters_involved.clone()).cast_as("json"),
            );
            word_count_target_case = word_count_target_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.word_count_target),
            );
            status_case = status_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.status.clone()),
            );
            updated_at_case = updated_at_case.case(
                chapter_outline::Column::Id.eq(outline.id),
                Expr::value(outline.updated_at),
            );
        }
        let updated = chapter_outline::Entity::update_many()
            .col_expr(
                chapter_outline::Column::PartId,
                part_id_case
                    .finally(Expr::col(chapter_outline::Column::PartId))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::Title,
                title_case
                    .finally(Expr::col(chapter_outline::Column::Title))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::Summary,
                summary_case
                    .finally(Expr::col(chapter_outline::Column::Summary))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::Beats,
                beats_case
                    .finally(Expr::col(chapter_outline::Column::Beats))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::KeyScenes,
                key_scenes_case
                    .finally(Expr::col(chapter_outline::Column::KeyScenes))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::PacingScore,
                pacing_score_case
                    .finally(Expr::col(chapter_outline::Column::PacingScore))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::ForeshadowingSetup,
                foreshadowing_setup_case
                    .finally(Expr::col(chapter_outline::Column::ForeshadowingSetup))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::ForeshadowingPayoff,
                foreshadowing_payoff_case
                    .finally(Expr::col(chapter_outline::Column::ForeshadowingPayoff))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::CharactersInvolved,
                characters_involved_case
                    .finally(Expr::col(chapter_outline::Column::CharactersInvolved))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::WordCountTarget,
                word_count_target_case
                    .finally(Expr::col(chapter_outline::Column::WordCountTarget))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::Status,
                status_case
                    .finally(Expr::col(chapter_outline::Column::Status))
                    .into(),
            )
            .col_expr(
                chapter_outline::Column::UpdatedAt,
                updated_at_case
                    .finally(Expr::col(chapter_outline::Column::UpdatedAt))
                    .into(),
            )
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .filter(chapter_outline::Column::Id.is_in(update_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != update_ids.len() as u64 {
            return Err(OutlineDbError::InvalidState {
                current: format!(
                    "章节大纲批量更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    update_ids.len()
                ),
                operation: "同步章节大纲".to_string(),
            });
        }
    }

    results
        .into_iter()
        .map(|result| {
            result.ok_or_else(|| OutlineDbError::InvalidState {
                current: "章节大纲批量插入后缺少返回记录".to_string(),
                operation: "同步章节大纲".to_string(),
            })
        })
        .map(|result| result.map(chapter_outline_record_from_model))
        .collect()
}
