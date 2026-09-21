use a4n_db_models::models::chapter::{chapter_outline, chapter_outline_provenance, part_outline};
use chrono::Utc;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, NotSet, QueryFilter, QueryOrder,
    TransactionTrait,
};
use std::collections::{HashMap, HashSet};

use crate::outline::{
    OutlineDbError, sync_chapter_outline_part_assignments, validate_part_outline_records,
};

use super::{ImportReverseDbError, ReverseOutlineApplyRow, ReversePartOutlineApplyRow};

pub async fn apply_reverse_outline_records<C>(
    db: &C,
    project_id: i64,
    rows: &[ReverseOutlineApplyRow],
) -> Result<usize, DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let chapter_numbers = rows
        .iter()
        .map(|row| row.chapter_number)
        .collect::<Vec<_>>();
    let mut existing_by_number = if chapter_numbers.is_empty() {
        HashMap::new()
    } else {
        chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .filter(chapter_outline::Column::ChapterNumber.is_in(chapter_numbers))
            .all(db)
            .await?
            .into_iter()
            .map(|outline| (outline.chapter_number, outline))
            .collect::<HashMap<_, _>>()
    };
    let existing_ids = existing_by_number
        .values()
        .map(|outline| outline.id)
        .collect::<Vec<_>>();
    if !existing_ids.is_empty() {
        chapter_outline_provenance::Entity::delete_many()
            .filter(chapter_outline_provenance::Column::OutlineId.is_in(existing_ids))
            .exec(db)
            .await?;
    }

    let mut touched_existing_ids = HashMap::new();
    let mut new_numbers = HashSet::new();
    for row in rows {
        if let Some(existing) = existing_by_number.get_mut(&row.chapter_number) {
            existing.updated_at =
                std::cmp::max(now, existing.updated_at + chrono::Duration::microseconds(1));
            existing.title = Some(row.title.clone());
            existing.summary = Some(row.summary.clone());
            if let Some(word_count_target) = row.word_count_target {
                existing.word_count_target = Some(word_count_target);
            }
            existing.status = "completed".to_string();
            if existing.id > 0 {
                touched_existing_ids.insert(existing.id, row.chapter_number);
            }
        } else {
            new_numbers.insert(row.chapter_number);
            existing_by_number.insert(
                row.chapter_number,
                chapter_outline::Model {
                    id: 0,
                    project_id,
                    part_id: None,
                    chapter_number: row.chapter_number,
                    title: Some(row.title.clone()),
                    summary: Some(row.summary.clone()),
                    beats: None,
                    key_scenes: None,
                    pacing_score: None,
                    foreshadowing_setup: None,
                    foreshadowing_payoff: None,
                    characters_involved: None,
                    word_count_target: row.word_count_target,
                    status: "completed".to_string(),
                    created_at: now,
                    updated_at: now,
                },
            );
        }
    }

    if !new_numbers.is_empty() {
        let mut inserts = Vec::with_capacity(new_numbers.len());
        for chapter_number in &new_numbers {
            let model = existing_by_number
                .get(chapter_number)
                .expect("new outline model must exist");
            let mut active: chapter_outline::ActiveModel = model.clone().into();
            active.id = NotSet;
            inserts.push(active);
        }
        let expected = inserts.len() as u64;
        let inserted = chapter_outline::Entity::insert_many(inserts)
            .exec_without_returning(db)
            .await?;
        if inserted != expected {
            return Err(DbErr::Custom(format!(
                "反向导入章节大纲批量插入返回行数不一致: inserted={inserted}, expected={expected}"
            )));
        }
    }

    if !touched_existing_ids.is_empty() {
        let mut title_case = CaseStatement::new();
        let mut summary_case = CaseStatement::new();
        let mut word_count_target_case = CaseStatement::new();
        let mut status_case = CaseStatement::new();
        let mut updated_at_case = CaseStatement::new();
        let ids = touched_existing_ids.keys().copied().collect::<Vec<_>>();
        for id in &ids {
            let number = touched_existing_ids
                .get(id)
                .expect("touched outline number must exist");
            let model = existing_by_number
                .get(number)
                .expect("touched outline model must exist");
            title_case = title_case.case(
                chapter_outline::Column::Id.eq(*id),
                Expr::value(model.title.clone()),
            );
            summary_case = summary_case.case(
                chapter_outline::Column::Id.eq(*id),
                Expr::value(model.summary.clone()),
            );
            word_count_target_case = word_count_target_case.case(
                chapter_outline::Column::Id.eq(*id),
                Expr::value(model.word_count_target),
            );
            status_case = status_case.case(
                chapter_outline::Column::Id.eq(*id),
                Expr::value(model.status.clone()),
            );
            updated_at_case = updated_at_case.case(
                chapter_outline::Column::Id.eq(*id),
                Expr::value(model.updated_at),
            );
        }
        let updated = chapter_outline::Entity::update_many()
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
            .filter(chapter_outline::Column::Id.is_in(ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(DbErr::Custom(format!(
                "反向导入章节大纲批量更新返回行数不一致: updated={}, expected={}",
                updated.rows_affected,
                ids.len()
            )));
        }
    }

    Ok(rows.len())
}

pub async fn apply_reverse_outline_records_transaction<T>(
    db: &T,
    project_id: i64,
    rows: &[ReverseOutlineApplyRow],
) -> Result<usize, DbErr>
where
    T: TransactionTrait,
{
    let txn = db.begin().await?;
    let outline_count = apply_reverse_outline_records(&txn, project_id, rows).await?;
    txn.commit().await?;
    Ok(outline_count)
}

pub async fn apply_reverse_part_outline_records<C>(
    db: &C,
    project_id: i64,
    rows: &[ReversePartOutlineApplyRow],
) -> Result<usize, ImportReverseDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let part_numbers = rows.iter().map(|row| row.part_number).collect::<Vec<_>>();
    let mut existing_by_number = if part_numbers.is_empty() {
        HashMap::new()
    } else {
        part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .filter(part_outline::Column::PartNumber.is_in(part_numbers))
            .all(db)
            .await?
            .into_iter()
            .map(|part| (part.part_number, part))
            .collect::<HashMap<_, _>>()
    };

    let mut touched_existing_ids = HashMap::new();
    let mut new_numbers = HashSet::new();
    for row in rows {
        if let Some(existing) = existing_by_number.get_mut(&row.part_number) {
            existing.updated_at =
                std::cmp::max(now, existing.updated_at + chrono::Duration::microseconds(1));
            existing.title = row.title.clone();
            existing.start_chapter = row.start_chapter;
            existing.end_chapter = row.end_chapter;
            existing.summary = Some(row.summary.clone());
            existing.theme = Some(row.theme.clone());
            existing.key_events = Some(row.key_events.clone());
            existing.character_focus = Some(row.character_focus.clone());
            if existing.id > 0 {
                touched_existing_ids.insert(existing.id, row.part_number);
            }
        } else {
            new_numbers.insert(row.part_number);
            existing_by_number.insert(
                row.part_number,
                part_outline::Model {
                    id: 0,
                    project_id,
                    part_number: row.part_number,
                    title: row.title.clone(),
                    start_chapter: row.start_chapter,
                    end_chapter: row.end_chapter,
                    summary: Some(row.summary.clone()),
                    theme: Some(row.theme.clone()),
                    key_events: Some(row.key_events.clone()),
                    character_focus: Some(row.character_focus.clone()),
                    created_at: now,
                    updated_at: now,
                },
            );
        }
    }

    if !new_numbers.is_empty() {
        let mut inserts = Vec::with_capacity(new_numbers.len());
        for part_number in &new_numbers {
            let model = existing_by_number
                .get(part_number)
                .expect("new part outline model must exist");
            let mut active: part_outline::ActiveModel = model.clone().into();
            active.id = NotSet;
            inserts.push(active);
        }
        let expected = inserts.len() as u64;
        let inserted = part_outline::Entity::insert_many(inserts)
            .exec_without_returning(db)
            .await?;
        if inserted != expected {
            return Err(ImportReverseDbError::InvalidArgument(format!(
                "反向导入分部大纲批量插入返回行数不一致: inserted={inserted}, expected={expected}"
            )));
        }
    }

    if !touched_existing_ids.is_empty() {
        let mut title_case = CaseStatement::new();
        let mut start_chapter_case = CaseStatement::new();
        let mut end_chapter_case = CaseStatement::new();
        let mut summary_case = CaseStatement::new();
        let mut theme_case = CaseStatement::new();
        let mut key_events_case = CaseStatement::new();
        let mut character_focus_case = CaseStatement::new();
        let mut updated_at_case = CaseStatement::new();
        let ids = touched_existing_ids.keys().copied().collect::<Vec<_>>();
        for id in &ids {
            let number = touched_existing_ids
                .get(id)
                .expect("touched part number must exist");
            let model = existing_by_number
                .get(number)
                .expect("touched part outline model must exist");
            title_case = title_case.case(
                part_outline::Column::Id.eq(*id),
                Expr::value(model.title.clone()),
            );
            start_chapter_case = start_chapter_case.case(
                part_outline::Column::Id.eq(*id),
                Expr::value(model.start_chapter),
            );
            end_chapter_case = end_chapter_case.case(
                part_outline::Column::Id.eq(*id),
                Expr::value(model.end_chapter),
            );
            summary_case = summary_case.case(
                part_outline::Column::Id.eq(*id),
                Expr::value(model.summary.clone()),
            );
            theme_case = theme_case.case(
                part_outline::Column::Id.eq(*id),
                Expr::value(model.theme.clone()),
            );
            key_events_case = key_events_case.case(
                part_outline::Column::Id.eq(*id),
                Expr::value(model.key_events.clone()).cast_as("json"),
            );
            character_focus_case = character_focus_case.case(
                part_outline::Column::Id.eq(*id),
                Expr::value(model.character_focus.clone()).cast_as("json"),
            );
            updated_at_case = updated_at_case.case(
                part_outline::Column::Id.eq(*id),
                Expr::value(model.updated_at),
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
            .await
            .map_err(ImportReverseDbError::Database)?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(ImportReverseDbError::InvalidArgument(format!(
                "反向导入分部大纲批量更新返回行数不一致: updated={}, expected={}",
                updated.rows_affected,
                ids.len()
            )));
        }
    }

    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await?;
    validate_part_outline_records(&parts).map_err(import_reverse_outline_error)?;
    sync_chapter_outline_part_assignments(db, project_id, &parts, now)
        .await
        .map_err(import_reverse_outline_error)?;

    Ok(rows.len())
}

pub async fn apply_reverse_part_outline_records_transaction<T>(
    db: &T,
    project_id: i64,
    rows: &[ReversePartOutlineApplyRow],
) -> Result<usize, ImportReverseDbError>
where
    T: TransactionTrait,
{
    let txn = db.begin().await?;
    let part_count = apply_reverse_part_outline_records(&txn, project_id, rows).await?;
    txn.commit().await?;
    Ok(part_count)
}

fn import_reverse_outline_error(error: OutlineDbError) -> ImportReverseDbError {
    match error {
        OutlineDbError::Database(error) => ImportReverseDbError::Database(error),
        other => ImportReverseDbError::InvalidArgument(other.to_string()),
    }
}
