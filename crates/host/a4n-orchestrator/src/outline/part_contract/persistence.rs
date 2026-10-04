use super::model::PartOutlineDraft;
use crate::outline::chapter_contract::{required_array, required_object, required_string};
use crate::outline::outline_db_error;
use a4n_db::ConnectionTrait;
use a4n_db::outline::{self as outline_db, PartOutlineSyncDraft};
use a4n_domain::chapter::PartOutlineRecord;
use a4n_error::AppError;
use serde_json::Value;

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
) -> Result<PartOutlineRecord, AppError> {
    let draft =
        part_outline_sync_draft_from_json(part_number, start_chapter, end_chapter, part_json)?;
    Ok(PartOutlineRecord {
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

pub async fn sync_part_outlines_and_assign_chapters_in<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<PartOutlineDraft>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<PartOutlineRecord>, AppError>
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

pub async fn insert_part_outline_from_json<C>(
    db: &C,
    project_id: i64,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<PartOutlineRecord, AppError>
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
    existing: PartOutlineRecord,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<PartOutlineRecord, AppError>
where
    C: ConnectionTrait,
{
    let draft =
        part_outline_sync_draft_from_json(part_number, start_chapter, end_chapter, part_json)?;
    outline_db::update_part_outline_record(db, existing, &draft, now)
        .await
        .map_err(outline_db_error)
}

pub async fn update_part_outline_and_rebind_chapters_in<C>(
    db: &C,
    project_id: i64,
    existing: PartOutlineRecord,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<PartOutlineRecord, AppError>
where
    C: ConnectionTrait,
{
    let draft =
        part_outline_sync_draft_from_json(part_number, start_chapter, end_chapter, part_json)?;
    outline_db::update_part_outline_and_rebind_chapters_in(db, project_id, existing, draft, now)
        .await
        .map_err(outline_db_error)
}
