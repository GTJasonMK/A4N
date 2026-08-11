use super::model::PartOutlineDraft;
use crate::outline::chapter_contract::{required_array, required_object, required_string};
use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::chapter::chapter_outline;
use a4n_core::models::chapter::part_outline;
use a4n_db::DbHandle;
use a4n_db::outline::{self as outline_db, OutlineDbError, PartOutlineSyncDraft};
use sea_orm::ConnectionTrait;
use serde_json::Value;

#[cfg(test)]
pub fn chapter_outline_ids_in_part_range(
    outlines: &[chapter_outline::Model],
    part: &part_outline::Model,
) -> Vec<i64> {
    outline_db::chapter_outline_ids_in_part_range(outlines, part)
}

#[cfg(test)]
pub fn chapter_outline_ids_in_part_ranges(
    outlines: &[chapter_outline::Model],
    parts: &[part_outline::Model],
) -> Vec<i64> {
    outline_db::chapter_outline_ids_in_part_ranges(outlines, parts)
}

pub fn part_outline_sync_draft_from_json(
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
) -> Result<PartOutlineSyncDraft, AppError> {
    Ok(PartOutlineSyncDraft {
        part_number,
        start_chapter,
        end_chapter,
        title: required_string(part_json, "title")?,
        summary: required_string(part_json, "summary")?,
        theme: required_string(part_json, "theme")?,
        key_events: required_array(part_json, "key_events")?,
        character_focus: required_object(part_json, "character_focus")?,
    })
}

pub fn part_outline_model_from_json(
    id: i64,
    project_id: i64,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<part_outline::Model, AppError> {
    let draft =
        part_outline_sync_draft_from_json(part_number, start_chapter, end_chapter, part_json)?;
    Ok(part_outline::Model {
        id,
        project_id,
        part_number: draft.part_number,
        title: draft.title,
        start_chapter: draft.start_chapter,
        end_chapter: draft.end_chapter,
        summary: Some(draft.summary),
        theme: Some(draft.theme),
        key_events: Some(draft.key_events),
        character_focus: Some(draft.character_focus),
        created_at: now,
        updated_at: now,
    })
}

#[cfg(test)]
pub async fn sync_part_outlines<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<PartOutlineDraft>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<part_outline::Model>, AppError>
where
    C: ConnectionTrait,
{
    let mut parsed = Vec::with_capacity(drafts.len());
    for draft in drafts {
        parsed.push(part_outline_sync_draft_from_json(
            draft.part_number,
            draft.start_chapter,
            draft.end_chapter,
            &draft.data,
        )?);
    }
    outline_db::sync_part_outline_records(db, project_id, parsed, now)
        .await
        .map_err(outline_db_error)
}

pub async fn sync_part_outlines_and_assign_chapters_transaction(
    db: &DbHandle,
    project_id: i64,
    drafts: Vec<PartOutlineDraft>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<part_outline::Model>, AppError> {
    let mut parsed = Vec::with_capacity(drafts.len());
    for draft in drafts {
        parsed.push(part_outline_sync_draft_from_json(
            draft.part_number,
            draft.start_chapter,
            draft.end_chapter,
            &draft.data,
        )?);
    }
    outline_db::sync_part_outline_records_and_assign_chapters_transaction(
        db.connection(),
        project_id,
        parsed,
        now,
    )
    .await
    .map_err(outline_db_error)
}

pub async fn sync_part_outlines_and_assign_chapters_in<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<PartOutlineDraft>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<part_outline::Model>, AppError>
where
    C: ConnectionTrait,
{
    let mut parsed = Vec::with_capacity(drafts.len());
    for draft in drafts {
        parsed.push(part_outline_sync_draft_from_json(
            draft.part_number,
            draft.start_chapter,
            draft.end_chapter,
            &draft.data,
        )?);
    }
    outline_db::sync_part_outline_records_and_assign_chapters_in(db, project_id, parsed, now)
        .await
        .map_err(outline_db_error)
}

#[cfg(test)]
pub async fn sync_chapter_outline_part_assignments<C>(
    db: &C,
    project_id: i64,
    parts: &[part_outline::Model],
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    outline_db::sync_chapter_outline_part_assignments(db, project_id, parts, now)
        .await
        .map_err(outline_db_error)
}

pub async fn insert_part_outline_from_json<C>(
    db: &C,
    project_id: i64,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<part_outline::Model, AppError>
where
    C: ConnectionTrait,
{
    let draft =
        part_outline_sync_draft_from_json(part_number, start_chapter, end_chapter, part_json)?;
    outline_db::insert_part_outline_record(db, project_id, &draft, now)
        .await
        .map_err(outline_db_error)
}

pub async fn update_part_outline_from_json<C>(
    db: &C,
    existing: part_outline::Model,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<part_outline::Model, AppError>
where
    C: ConnectionTrait,
{
    let draft =
        part_outline_sync_draft_from_json(part_number, start_chapter, end_chapter, part_json)?;
    outline_db::update_part_outline_record(db, existing, &draft, now)
        .await
        .map_err(outline_db_error)
}

pub async fn update_part_outline_and_rebind_chapters_transaction(
    db: &DbHandle,
    project_id: i64,
    existing: part_outline::Model,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<part_outline::Model, AppError> {
    let draft =
        part_outline_sync_draft_from_json(part_number, start_chapter, end_chapter, part_json)?;
    outline_db::update_part_outline_and_rebind_chapters_transaction(
        db.connection(),
        project_id,
        existing,
        draft,
        now,
    )
    .await
    .map_err(outline_db_error)
}

pub async fn update_part_outline_and_rebind_chapters_in<C>(
    db: &C,
    project_id: i64,
    existing: part_outline::Model,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<part_outline::Model, AppError>
where
    C: ConnectionTrait,
{
    let draft =
        part_outline_sync_draft_from_json(part_number, start_chapter, end_chapter, part_json)?;
    outline_db::update_part_outline_and_rebind_chapters_in(db, project_id, existing, draft, now)
        .await
        .map_err(outline_db_error)
}

fn outline_db_error(error: OutlineDbError) -> AppError {
    match error {
        OutlineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        OutlineDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        OutlineDbError::Database(err) => AppError::Database(err),
        OutlineDbError::Generation(err) => crate::generation::generation_error(err),
    }
}
