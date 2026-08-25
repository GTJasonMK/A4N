use std::collections::{BTreeMap, HashSet};

use a4n_core::models::chapter::part_outline;
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::super::{OutlineDbError, PartOutlineSyncDraft};
use super::records::{part_outline_active_model, part_outline_matches};

/// 按 part_number 增量同步分部大纲：保留匹配 id、更新变更、插入缺失、删除多余。
pub async fn sync_part_outline_records<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<PartOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<part_outline::Model>, OutlineDbError>
where
    C: ConnectionTrait,
{
    if drafts.is_empty() {
        return Ok(Vec::new());
    }

    let mut seen = HashSet::new();
    for draft in &drafts {
        if !seen.insert(draft.part_number) {
            return Err(OutlineDbError::InvalidArgument(format!(
                "分部大纲重复: 第 {} 部",
                draft.part_number
            )));
        }
    }

    let existing = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await?;
    let mut existing_by_number = BTreeMap::<i32, Vec<part_outline::Model>>::new();
    for part in existing {
        existing_by_number
            .entry(part.part_number)
            .or_default()
            .push(part);
    }

    let mut results = Vec::with_capacity(drafts.len());
    let mut pending_inserts = Vec::new();
    let mut pending_insert_slots = Vec::new();
    let mut pending_updates = Vec::new();
    for draft in drafts {
        let result_slot = results.len();
        if let Some(candidates) = existing_by_number.get_mut(&draft.part_number)
            && !candidates.is_empty()
        {
            let existing = candidates.remove(0);
            if part_outline_matches(&existing, &draft) {
                results.push(Some(existing));
                continue;
            }
            let updated_at =
                std::cmp::max(now, existing.updated_at + chrono::Duration::microseconds(1));
            let mut updated = existing;
            updated.title = draft.title;
            updated.start_chapter = draft.start_chapter;
            updated.end_chapter = draft.end_chapter;
            updated.summary = Some(draft.summary);
            updated.theme = Some(draft.theme);
            updated.key_events = Some(draft.key_events);
            updated.character_focus = Some(draft.character_focus);
            updated.updated_at = updated_at;
            pending_updates.push(updated.clone());
            results.push(Some(updated));
        } else {
            pending_inserts.push(part_outline_active_model(project_id, &draft, now));
            pending_insert_slots.push((result_slot, draft.part_number));
            results.push(None);
        }
    }

    if !pending_inserts.is_empty() {
        let expected = pending_inserts.len();
        let inserted = part_outline::Entity::insert_many(pending_inserts)
            .exec_with_returning_many(db)
            .await?;
        if inserted.len() != expected {
            return Err(OutlineDbError::InvalidArgument(format!(
                "分部大纲批量插入返回行数不一致: inserted={}, expected={}",
                inserted.len(),
                expected
            )));
        }
        let inserted_by_number = inserted
            .into_iter()
            .map(|part| (part.part_number, part))
            .collect::<BTreeMap<_, _>>();
        for (slot, part_number) in pending_insert_slots {
            results[slot] = inserted_by_number.get(&part_number).cloned();
        }
    }

    if !pending_updates.is_empty() {
        let mut title_case = CaseStatement::new();
        let mut start_chapter_case = CaseStatement::new();
        let mut end_chapter_case = CaseStatement::new();
        let mut summary_case = CaseStatement::new();
        let mut theme_case = CaseStatement::new();
        let mut key_events_case = CaseStatement::new();
        let mut character_focus_case = CaseStatement::new();
        let mut updated_at_case = CaseStatement::new();
        let ids = pending_updates
            .iter()
            .map(|part| part.id)
            .collect::<Vec<_>>();
        for part in &pending_updates {
            title_case = title_case.case(
                part_outline::Column::Id.eq(part.id),
                Expr::value(part.title.clone()),
            );
            start_chapter_case = start_chapter_case.case(
                part_outline::Column::Id.eq(part.id),
                Expr::value(part.start_chapter),
            );
            end_chapter_case = end_chapter_case.case(
                part_outline::Column::Id.eq(part.id),
                Expr::value(part.end_chapter),
            );
            summary_case = summary_case.case(
                part_outline::Column::Id.eq(part.id),
                Expr::value(part.summary.clone()),
            );
            theme_case = theme_case.case(
                part_outline::Column::Id.eq(part.id),
                Expr::value(part.theme.clone()),
            );
            key_events_case = key_events_case.case(
                part_outline::Column::Id.eq(part.id),
                Expr::value(part.key_events.clone()).cast_as("json"),
            );
            character_focus_case = character_focus_case.case(
                part_outline::Column::Id.eq(part.id),
                Expr::value(part.character_focus.clone()).cast_as("json"),
            );
            updated_at_case = updated_at_case.case(
                part_outline::Column::Id.eq(part.id),
                Expr::value(part.updated_at),
            );
        }
        let updated = part_outline::Entity::update_many()
            .col_expr(
                part_outline::Column::Title,
                title_case
                    .finally(Expr::col(part_outline::Column::Title))
                    .into(),
            )
            .col_expr(
                part_outline::Column::StartChapter,
                start_chapter_case
                    .finally(Expr::col(part_outline::Column::StartChapter))
                    .into(),
            )
            .col_expr(
                part_outline::Column::EndChapter,
                end_chapter_case
                    .finally(Expr::col(part_outline::Column::EndChapter))
                    .into(),
            )
            .col_expr(
                part_outline::Column::Summary,
                summary_case
                    .finally(Expr::col(part_outline::Column::Summary))
                    .into(),
            )
            .col_expr(
                part_outline::Column::Theme,
                theme_case
                    .finally(Expr::col(part_outline::Column::Theme))
                    .into(),
            )
            .col_expr(
                part_outline::Column::KeyEvents,
                key_events_case
                    .finally(Expr::col(part_outline::Column::KeyEvents))
                    .into(),
            )
            .col_expr(
                part_outline::Column::CharacterFocus,
                character_focus_case
                    .finally(Expr::col(part_outline::Column::CharacterFocus))
                    .into(),
            )
            .col_expr(
                part_outline::Column::UpdatedAt,
                updated_at_case
                    .finally(Expr::col(part_outline::Column::UpdatedAt))
                    .into(),
            )
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .filter(part_outline::Column::Id.is_in(ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(OutlineDbError::InvalidState {
                current: format!(
                    "分部大纲批量更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    ids.len()
                ),
                operation: "同步分部大纲".to_string(),
            });
        }
    }

    let stale_ids = existing_by_number
        .into_values()
        .flatten()
        .map(|stale| stale.id)
        .collect::<Vec<_>>();
    if !stale_ids.is_empty() {
        part_outline::Entity::delete_many()
            .filter(part_outline::Column::Id.is_in(stale_ids))
            .exec(db)
            .await?;
    }

    results
        .into_iter()
        .map(|result| {
            result.ok_or_else(|| OutlineDbError::InvalidState {
                current: "分部大纲批量插入后缺少返回记录".to_string(),
                operation: "同步分部大纲".to_string(),
            })
        })
        .collect()
}
