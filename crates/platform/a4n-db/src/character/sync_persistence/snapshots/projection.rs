use a4n_db_models::models::character::attribute_event;
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::character::CharacterDbError;

pub fn character_sync_attribute_events_at_chapter(
    events: &[attribute_event::Model],
    chapter_number: i32,
) -> Value {
    let by_chapter = character_sync_attribute_events_by_chapter(events);
    character_sync_attribute_events_from_index(&by_chapter, chapter_number)
}

pub(super) type CharacterSyncEventsByChapter<'a> = HashMap<i32, Vec<&'a attribute_event::Model>>;

pub(super) fn character_sync_attribute_events_by_chapter(
    events: &[attribute_event::Model],
) -> CharacterSyncEventsByChapter<'_> {
    let mut by_chapter: CharacterSyncEventsByChapter<'_> = HashMap::new();
    for event in events {
        by_chapter
            .entry(event.chapter_number)
            .or_default()
            .push(event);
    }
    by_chapter
}

pub(super) fn character_sync_attribute_events_from_index(
    events_by_chapter: &CharacterSyncEventsByChapter<'_>,
    chapter_number: i32,
) -> Value {
    let events = events_by_chapter
        .get(&chapter_number)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    Value::Array(
        events
            .iter()
            .map(|event| {
                json!({
                    "id": event.id,
                    "chapter_number": event.chapter_number,
                    "source_kind": event.source_kind.clone(),
                    "timestamp": event.timestamp.to_string(),
                    "category": event.category.clone(),
                    "key": event.key.clone(),
                    "operation": event.operation.clone(),
                    "change_description": event.change_description.clone(),
                    "old_value": event.old_value.clone(),
                    "new_value": event.new_value.clone(),
                    "cause": event.cause.clone(),
                    "evidence": event.evidence.clone(),
                })
            })
            .collect(),
    )
}

pub(super) type CharacterSyncItemsByChapter<'a> = HashMap<i32, Vec<&'a Value>>;

pub(super) fn character_sync_items_by_chapter<'a>(
    items: &'a [Value],
) -> Result<CharacterSyncItemsByChapter<'a>, CharacterDbError> {
    let mut by_chapter: CharacterSyncItemsByChapter<'a> = HashMap::new();
    for item in items {
        let chapter_number = character_sync_required_i32(item, "chapter_number")?;
        by_chapter.entry(chapter_number).or_default().push(item);
    }
    Ok(by_chapter)
}

pub(super) fn character_sync_items_from_index(
    items_by_chapter: &CharacterSyncItemsByChapter<'_>,
    chapter_number: i32,
) -> Value {
    let items = items_by_chapter
        .get(&chapter_number)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    Value::Array(items.iter().map(|item| (*item).clone()).collect())
}

pub(super) fn character_sync_item_count_from_index(
    items_by_chapter: &CharacterSyncItemsByChapter<'_>,
    chapter_number: i32,
) -> usize {
    items_by_chapter.get(&chapter_number).map_or(0, Vec::len)
}

pub(in crate::character::sync_persistence) fn character_sync_required_i32(
    item: &Value,
    field: &str,
) -> Result<i32, CharacterDbError> {
    let value = item
        .get(field)
        .ok_or_else(|| CharacterDbError::InvalidArgument(format!("缺少字段 {field}")))?;
    let number = value
        .as_i64()
        .ok_or_else(|| CharacterDbError::InvalidArgument(format!("字段 {field} 必须是整数")))?;
    i32::try_from(number)
        .map_err(|_| CharacterDbError::InvalidArgument(format!("字段 {field} 超出 i32 范围")))
}
