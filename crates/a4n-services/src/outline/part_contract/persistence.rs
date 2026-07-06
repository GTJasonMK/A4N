use super::model::PartOutlineDraft;
use crate::generation::delete_generation_run_targets_for;
use crate::outline::chapter_contract::{
    requested_part_for_chapter, required_array, required_object, required_string,
};
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use sea_orm::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::outline) fn part_outline_active_model(
    project_id: i64,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<part_outline::ActiveModel, AppError> {
    Ok(part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(part_number),
        title: Set(required_string(part_json, "title")?),
        start_chapter: Set(start_chapter),
        end_chapter: Set(end_chapter),
        summary: Set(Some(required_string(part_json, "summary")?)),
        theme: Set(Some(required_string(part_json, "theme")?)),
        key_events: Set(Some(required_array(part_json, "key_events")?)),
        character_focus: Set(Some(required_object(part_json, "character_focus")?)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
}

pub(in crate::outline) fn part_outline_model_from_json(
    id: i64,
    project_id: i64,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<part_outline::Model, AppError> {
    Ok(part_outline::Model {
        id,
        project_id,
        part_number,
        title: required_string(part_json, "title")?,
        start_chapter,
        end_chapter,
        summary: Some(required_string(part_json, "summary")?),
        theme: Some(required_string(part_json, "theme")?),
        key_events: Some(required_array(part_json, "key_events")?),
        character_focus: Some(required_object(part_json, "character_focus")?),
        created_at: now,
        updated_at: now,
    })
}

pub(in crate::outline) async fn sync_part_outlines<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<PartOutlineDraft>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<part_outline::Model>, AppError>
where
    C: ConnectionTrait,
{
    let mut seen = BTreeSet::new();
    for draft in &drafts {
        if !seen.insert(draft.part_number) {
            return Err(AppError::InvalidArgument(format!(
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
        .await
        .map_err(AppError::Database)?;
    let mut existing_by_number = BTreeMap::<i32, Vec<part_outline::Model>>::new();
    for part in existing {
        existing_by_number
            .entry(part.part_number)
            .or_default()
            .push(part);
    }

    let mut results = Vec::with_capacity(drafts.len());
    for draft in drafts {
        if let Some(candidates) = existing_by_number.get_mut(&draft.part_number)
            && !candidates.is_empty()
        {
            let existing = candidates.remove(0);
            if part_outline_matches(
                &existing,
                draft.part_number,
                draft.start_chapter,
                draft.end_chapter,
                &draft.data,
            )? {
                results.push(existing);
                continue;
            }
            let active = update_part_outline_active_model(
                existing,
                draft.part_number,
                draft.start_chapter,
                draft.end_chapter,
                &draft.data,
                now,
            )?;
            let updated = active.update(db).await.map_err(AppError::Database)?;
            results.push(updated);
        } else {
            let inserted = part_outline_active_model(
                project_id,
                draft.part_number,
                draft.start_chapter,
                draft.end_chapter,
                &draft.data,
                now,
            )?
            .insert(db)
            .await
            .map_err(AppError::Database)?;
            results.push(inserted);
        }
    }

    for stale in existing_by_number.into_values().flatten() {
        part_outline::Entity::delete_by_id(stale.id)
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }

    Ok(results)
}

pub(in crate::outline) async fn sync_chapter_outline_part_assignments<C>(
    db: &C,
    project_id: i64,
    parts: &[part_outline::Model],
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    for outline in outlines {
        let expected_part_id = expected_part_id_for_existing_outline(parts, &outline)?;
        if outline.part_id == expected_part_id {
            continue;
        }
        let mut active: chapter_outline::ActiveModel = outline.into();
        active.part_id = Set(expected_part_id);
        active.updated_at = Set(now);
        active.update(db).await.map_err(AppError::Database)?;
    }

    Ok(())
}

fn expected_part_id_for_existing_outline(
    parts: &[part_outline::Model],
    outline: &chapter_outline::Model,
) -> Result<Option<i64>, AppError> {
    if parts.is_empty() {
        return Ok(None);
    }
    requested_part_for_chapter(parts, outline.chapter_number)?
        .map(|part| Some(part.id))
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "已有第 {} 章章纲不在任何分部范围内，请先删除旧章纲或重新生成覆盖该章节的分部",
                outline.chapter_number
            ))
        })
}

pub(in crate::outline) async fn delete_chapter_outlines_in_part_range<C>(
    db: &C,
    project_id: i64,
    part: &part_outline::Model,
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.gte(part.start_chapter))
        .filter(chapter_outline::Column::ChapterNumber.lte(part.end_chapter))
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .collect::<Vec<_>>();
    let outline_ids = chapter_outline_ids_in_part_range(&outlines, part);
    if outline_ids.is_empty() {
        return Ok(outline_ids);
    }

    chapter_outline::Entity::delete_many()
        .filter(chapter_outline::Column::Id.is_in(outline_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    delete_generation_run_targets_for(db, "chapter_outline", &outline_ids).await?;
    Ok(outline_ids)
}

pub(in crate::outline) fn chapter_outline_ids_in_part_range(
    outlines: &[chapter_outline::Model],
    part: &part_outline::Model,
) -> Vec<i64> {
    outlines
        .iter()
        .filter(|outline| {
            outline.chapter_number >= part.start_chapter
                && outline.chapter_number <= part.end_chapter
        })
        .map(|outline| outline.id)
        .collect()
}

pub(in crate::outline) fn chapter_outline_ids_in_part_ranges(
    outlines: &[chapter_outline::Model],
    parts: &[part_outline::Model],
) -> Vec<i64> {
    let mut ids = BTreeSet::new();
    for part in parts {
        ids.extend(chapter_outline_ids_in_part_range(outlines, part));
    }
    ids.into_iter().collect()
}

pub(in crate::outline) fn update_part_outline_active_model(
    old: part_outline::Model,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
    now: chrono::NaiveDateTime,
) -> Result<part_outline::ActiveModel, AppError> {
    let mut active: part_outline::ActiveModel = old.into();
    active.part_number = Set(part_number);
    active.title = Set(required_string(part_json, "title")?);
    active.start_chapter = Set(start_chapter);
    active.end_chapter = Set(end_chapter);
    active.summary = Set(Some(required_string(part_json, "summary")?));
    active.theme = Set(Some(required_string(part_json, "theme")?));
    active.key_events = Set(Some(required_array(part_json, "key_events")?));
    active.character_focus = Set(Some(required_object(part_json, "character_focus")?));
    active.updated_at = Set(now);
    Ok(active)
}

fn part_outline_matches(
    existing: &part_outline::Model,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    part_json: &Value,
) -> Result<bool, AppError> {
    let title = required_string(part_json, "title")?;
    let summary = required_string(part_json, "summary")?;
    let theme = required_string(part_json, "theme")?;
    let key_events = required_array(part_json, "key_events")?;
    let character_focus = required_object(part_json, "character_focus")?;

    Ok(existing.part_number == part_number
        && existing.start_chapter == start_chapter
        && existing.end_chapter == end_chapter
        && existing.title == title
        && existing.summary.as_deref() == Some(summary.as_str())
        && existing.theme.as_deref() == Some(theme.as_str())
        && existing.key_events == Some(key_events)
        && existing.character_focus == Some(character_focus))
}
