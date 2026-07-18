use a4n_core::models::character::attribute_event;
use serde_json::{Value, json};

use crate::character::CharacterDbError;

pub fn character_sync_attribute_events_at_chapter(
    events: &[attribute_event::Model],
    chapter_number: i32,
) -> Value {
    Value::Array(
        events
            .iter()
            .filter(|event| event.chapter_number == chapter_number)
            .map(|event| {
                json!({
                    "id": event.id,
                    "chapter_number": event.chapter_number,
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

pub(super) fn character_sync_items_at_chapter(
    items: &[Value],
    chapter_number: i32,
) -> Result<Value, CharacterDbError> {
    let mut matched = Vec::new();
    for item in items {
        if character_sync_required_i32(item, "chapter_number")? == chapter_number {
            matched.push(item.clone());
        }
    }
    Ok(Value::Array(matched))
}

pub(super) fn character_sync_count_items_at_chapter(
    items: &[Value],
    chapter_number: i32,
) -> Result<usize, CharacterDbError> {
    let mut count = 0;
    for item in items {
        if character_sync_required_i32(item, "chapter_number")? == chapter_number {
            count += 1;
        }
    }
    Ok(count)
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
