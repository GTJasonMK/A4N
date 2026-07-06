use crate::state::AppState;
use a4n_core::models::character::{protagonist_profile, snapshot_reference};
use a4n_services::character::snapshot_attributes_from_data;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;

use super::json_utils::{
    category_storage_key, required_attribute_change_operation, required_category, required_i32,
    required_string, required_usize, required_value,
};
use super::sync_write::AttributeEventDraft;

pub(super) fn project_attribute_changes(
    mut state: Value,
    changes: &[Value],
) -> Result<Value, String> {
    for change in changes {
        apply_sync_change_to_state(&mut state, change)?;
    }
    Ok(state)
}

pub(super) fn apply_sync_change_to_state(state: &mut Value, change: &Value) -> Result<(), String> {
    let mut event = attribute_event_from_change(0, change)?;
    fill_event_old_value_and_apply(state, &mut event)
}

pub(super) fn pending_attribute_events(
    baseline_state: &Value,
    changes: &[Value],
    classification_suggestions: &[Value],
) -> Result<Vec<AttributeEventDraft>, String> {
    let mut events = Vec::with_capacity(changes.len() + classification_suggestions.len());
    for (index, change) in changes.iter().enumerate() {
        events.push(attribute_event_from_change(index, change)?);
    }
    for suggestion in classification_suggestions {
        events.push(attribute_event_from_classification_suggestion(suggestion)?);
    }
    events.sort_by(|left, right| {
        left.chapter_number
            .cmp(&right.chapter_number)
            .then_with(|| left.stage.cmp(&right.stage))
            .then_with(|| left.sequence.cmp(&right.sequence))
    });
    let mut projected = baseline_state.clone();
    for event in &mut events {
        fill_event_old_value_and_apply(&mut projected, event)?;
    }
    Ok(events)
}

fn attribute_event_from_change(
    index: usize,
    change: &Value,
) -> Result<AttributeEventDraft, String> {
    Ok(AttributeEventDraft {
        chapter_number: required_i32(change, "chapter_number")?,
        sequence: index,
        stage: 0,
        category: required_category(change, "category")?,
        key: required_string(change, "key")?,
        operation: required_attribute_change_operation(change, "operation")?,
        old_value: Value::Null,
        new_value: required_value(change, "new_value")?.clone(),
        change_description: required_string(change, "change_description")?,
        cause: required_string(change, "event_cause")?,
        evidence: required_string(change, "evidence")?,
        from_classification: false,
    })
}

fn attribute_event_from_classification_suggestion(
    suggestion: &Value,
) -> Result<AttributeEventDraft, String> {
    let reasoning = required_string(suggestion, "reasoning")?;
    let key = required_string(suggestion, "key")?;
    Ok(AttributeEventDraft {
        chapter_number: required_i32(suggestion, "chapter_number")?,
        sequence: required_usize(suggestion, "sequence")?,
        stage: 1,
        category: "implicit".to_string(),
        key: key.clone(),
        operation: "add".to_string(),
        old_value: Value::Null,
        new_value: required_value(suggestion, "value")?.clone(),
        change_description: format!("从行为分类推断新增隐性属性: {key}"),
        cause: format!(
            "LLM从行为分析中推断: {}",
            reasoning.chars().take(100).collect::<String>()
        ),
        evidence: required_string(suggestion, "evidence")?,
        from_classification: true,
    })
}

fn fill_event_old_value_and_apply(
    state: &mut Value,
    event: &mut AttributeEventDraft,
) -> Result<(), String> {
    let existing = attribute_value_in_state(state, &event.category, &event.key);
    match event.operation.as_str() {
        "add" => {
            if existing.is_some() {
                return Err(format!(
                    "属性事件不一致：第 {} 章 add {}.{}，但该属性已存在",
                    event.chapter_number, event.category, event.key
                ));
            }
            if event.new_value.is_null() {
                return Err(format!(
                    "属性事件不一致：第 {} 章 add {}.{} 的 new_value 不能为空",
                    event.chapter_number, event.category, event.key
                ));
            }
            event.old_value = Value::Null;
        }
        "modify" => {
            let old_value = existing.ok_or_else(|| {
                format!(
                    "属性事件不一致：第 {} 章 modify {}.{}，但该属性不存在",
                    event.chapter_number, event.category, event.key
                )
            })?;
            if event.new_value.is_null() {
                return Err(format!(
                    "属性事件不一致：第 {} 章 modify {}.{} 的 new_value 不能为空",
                    event.chapter_number, event.category, event.key
                ));
            }
            event.old_value = old_value;
        }
        "delete" => {
            let old_value = existing.ok_or_else(|| {
                format!(
                    "属性事件不一致：第 {} 章 delete {}.{}，但该属性不存在",
                    event.chapter_number, event.category, event.key
                )
            })?;
            if !event.new_value.is_null() {
                return Err(format!(
                    "属性事件不一致：第 {} 章 delete {}.{} 的 new_value 必须为 null",
                    event.chapter_number, event.category, event.key
                ));
            }
            event.old_value = old_value;
        }
        _ => {
            return Err(format!("未知属性事件 operation: {}", event.operation));
        }
    }
    apply_attribute_to_state(
        state,
        &event.category,
        &event.key,
        &event.operation,
        event.new_value.clone(),
    )?;
    Ok(())
}

fn attribute_value_in_state(state: &Value, category: &str, key: &str) -> Option<Value> {
    state
        .get(category_storage_key(category))
        .and_then(Value::as_object)
        .and_then(|attributes| attributes.get(key))
        .cloned()
}

pub(super) async fn profile_state_before_chapter(
    state: &AppState,
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> Result<Value, String> {
    if chapter_number <= 1 {
        return Ok(profile.base_attributes.clone());
    }

    let snapshot_chapter = chapter_number - 1;
    let snapshot = snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(profile.id))
        .filter(snapshot_reference::Column::ChapterNumber.eq(snapshot_chapter))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!("第 {snapshot_chapter} 章没有角色快照，无法从第 {chapter_number} 章断点重同步")
        })?;

    snapshot_attributes_from_data(&snapshot.data, snapshot_chapter).map_err(|e| e.to_string())
}

pub(super) fn apply_attribute_to_state(
    state: &mut Value,
    category: &str,
    key: &str,
    operation: &str,
    new_value: Value,
) -> Result<(), String> {
    let storage_key = category_storage_key(category);
    let root = state
        .as_object_mut()
        .ok_or_else(|| "角色当前状态必须是 JSON 对象".to_string())?;
    let attributes = root
        .get_mut(storage_key)
        .ok_or_else(|| format!("角色当前状态缺少 {storage_key}"))?
        .as_object_mut()
        .ok_or_else(|| format!("角色当前状态 {storage_key} 必须是 JSON 对象"))?;
    if operation == "delete" {
        attributes.remove(key);
    } else {
        attributes.insert(key.to_string(), new_value);
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn fold_attribute_event_tuples(
    baseline_state: Value,
    events: &mut [(i32, i64, &str, &str, &str, Value)],
    until_chapter: i32,
) -> Value {
    events.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let mut state = baseline_state;
    for (chapter_number, _, category, key, operation, new_value) in events.iter() {
        if *chapter_number > until_chapter {
            break;
        }
        apply_attribute_to_state(&mut state, category, key, operation, new_value.clone())
            .expect("test state must be canonical");
    }
    state
}
