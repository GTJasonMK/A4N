use super::fields::{optional_f32, optional_non_null, optional_positive_i32, required_string};
use a4n_db::outline::{self as outline_db, OutlineDbError};
use a4n_domain::chapter::{ChapterOutlineRecord, ChapterOutlineSyncDraft};
use a4n_error::AppError;
use sea_orm::ConnectionTrait;
use serde_json::Value;

fn chapter_outline_sync_draft_from_json(
    chapter_json: &Value,
) -> Result<ChapterOutlineSyncDraft, AppError> {
    Ok(ChapterOutlineSyncDraft {
        chapter_number: chapter_outline_number(chapter_json)?,
        title: required_string(chapter_json, "title")?,
        summary: required_string(chapter_json, "summary")?,
        beats: optional_non_null(chapter_json, "beats"),
        key_scenes: optional_non_null(chapter_json, "key_scenes"),
        pacing_score: optional_f32(chapter_json, "pacing_score")?,
        foreshadowing_setup: optional_non_null(chapter_json, "foreshadowing_setup"),
        foreshadowing_payoff: optional_non_null(chapter_json, "foreshadowing_payoff"),
        characters_involved: optional_non_null(chapter_json, "characters_involved"),
        word_count_target: optional_positive_i32(chapter_json, "word_count_target")?,
    })
}

fn chapter_outline_number(chapter_json: &Value) -> Result<i32, AppError> {
    let chapter_number = chapter_json
        .get("chapter_number")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            AppError::InvalidArgument("大纲字段 chapter_number 必须是整数".to_string())
        })?;
    i32::try_from(chapter_number)
        .map_err(|_| AppError::InvalidArgument("大纲字段 chapter_number 超出 i32 范围".to_string()))
}

pub(in crate::outline) async fn sync_chapter_outlines_by_number_in<C>(
    db: &C,
    project_id: i64,
    part_id: Option<i64>,
    chapters: Vec<Value>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<ChapterOutlineRecord>, AppError>
where
    C: ConnectionTrait,
{
    let mut drafts = Vec::with_capacity(chapters.len());
    for chapter_json in &chapters {
        drafts.push(chapter_outline_sync_draft_from_json(chapter_json)?);
    }
    outline_db::sync_chapter_outline_records_by_number(db, project_id, part_id, drafts, now)
        .await
        .map_err(outline_db_error)
}

pub(in crate::outline) async fn insert_chapter_outline_from_json<C>(
    db: &C,
    project_id: i64,
    part_id: Option<i64>,
    chapter_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<ChapterOutlineRecord, AppError>
where
    C: ConnectionTrait,
{
    let draft = chapter_outline_sync_draft_from_json(chapter_json)?;
    outline_db::insert_chapter_outline_record(db, project_id, part_id, &draft, now)
        .await
        .map_err(outline_db_error)
}

pub(in crate::outline) async fn update_chapter_outline_from_json_in<C>(
    db: &C,
    existing: ChapterOutlineRecord,
    part_id: Option<i64>,
    chapter_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<ChapterOutlineRecord, AppError>
where
    C: ConnectionTrait,
{
    let draft = chapter_outline_sync_draft_from_json(chapter_json)?;
    outline_db::update_chapter_outline_record(db, existing, part_id, &draft, now)
        .await
        .map_err(outline_db_error)
}

pub(in crate::outline) fn outline_db_error(error: OutlineDbError) -> AppError {
    match error {
        OutlineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        OutlineDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        OutlineDbError::Database(err) => AppError::Database(err),
        OutlineDbError::Generation(err) => crate::generation::generation_error(err),
    }
}
