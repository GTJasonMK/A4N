use super::fields::{optional_f32, optional_non_null, optional_positive_i32, required_string};
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use sea_orm::*;
use serde_json::Value;
use std::collections::BTreeMap;

pub(in crate::outline) fn chapter_outline_active_model(
    project_id: i64,
    part_id: Option<i64>,
    chapter_json: Value,
    now: chrono::NaiveDateTime,
) -> Result<chapter_outline::ActiveModel, AppError> {
    let chapter_number = chapter_outline_number(&chapter_json)?;
    Ok(chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(part_id),
        chapter_number: Set(chapter_number),
        title: Set(Some(required_string(&chapter_json, "title")?)),
        summary: Set(Some(required_string(&chapter_json, "summary")?)),
        beats: Set(optional_non_null(&chapter_json, "beats")),
        key_scenes: Set(optional_non_null(&chapter_json, "key_scenes")),
        pacing_score: Set(optional_f32(&chapter_json, "pacing_score")?),
        foreshadowing_setup: Set(optional_non_null(&chapter_json, "foreshadowing_setup")),
        foreshadowing_payoff: Set(optional_non_null(&chapter_json, "foreshadowing_payoff")),
        characters_involved: Set(optional_non_null(&chapter_json, "characters_involved")),
        word_count_target: Set(optional_positive_i32(&chapter_json, "word_count_target")?),
        status: Set("draft".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
}

pub(in crate::outline) fn update_chapter_outline_active_model(
    old: chapter_outline::Model,
    part_id: Option<i64>,
    chapter_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<chapter_outline::ActiveModel, AppError> {
    let mut active: chapter_outline::ActiveModel = old.into();
    active.part_id = Set(part_id);
    active.title = Set(Some(required_string(chapter_json, "title")?));
    active.summary = Set(Some(required_string(chapter_json, "summary")?));
    active.beats = Set(optional_non_null(chapter_json, "beats"));
    active.key_scenes = Set(optional_non_null(chapter_json, "key_scenes"));
    active.pacing_score = Set(optional_f32(chapter_json, "pacing_score")?);
    active.foreshadowing_setup = Set(optional_non_null(chapter_json, "foreshadowing_setup"));
    active.foreshadowing_payoff = Set(optional_non_null(chapter_json, "foreshadowing_payoff"));
    active.characters_involved = Set(optional_non_null(chapter_json, "characters_involved"));
    active.word_count_target = Set(optional_positive_i32(chapter_json, "word_count_target")?);
    active.status = Set("draft".to_string());
    active.updated_at = Set(now);
    Ok(active)
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

pub(in crate::outline) async fn sync_chapter_outlines_by_number<C>(
    db: &C,
    project_id: i64,
    part_id: Option<i64>,
    chapters: Vec<Value>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<chapter_outline::Model>, AppError>
where
    C: ConnectionTrait,
{
    let mut chapter_numbers = Vec::with_capacity(chapters.len());
    for chapter_json in &chapters {
        chapter_numbers.push(chapter_outline_number(chapter_json)?);
    }

    let existing = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.is_in(chapter_numbers))
        .all(db)
        .await
        .map_err(AppError::Database)?;

    let mut existing_by_number = BTreeMap::<i32, chapter_outline::Model>::new();
    for outline in existing {
        if let Some(previous) = existing_by_number.insert(outline.chapter_number, outline.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "第 {} 章已有重复章纲记录：{}, {}，请先清理后再应用节拍映射",
                outline.chapter_number, previous.id, outline.id
            )));
        }
    }

    let mut results = Vec::with_capacity(chapters.len());
    for chapter_json in chapters {
        let chapter_number = chapter_outline_number(&chapter_json)?;
        let outline = if let Some(existing) = existing_by_number.remove(&chapter_number) {
            update_chapter_outline_active_model(existing, part_id, &chapter_json, now)?
                .update(db)
                .await
                .map_err(AppError::Database)?
        } else {
            chapter_outline_active_model(project_id, part_id, chapter_json, now)?
                .insert(db)
                .await
                .map_err(AppError::Database)?
        };
        results.push(outline);
    }

    Ok(results)
}
