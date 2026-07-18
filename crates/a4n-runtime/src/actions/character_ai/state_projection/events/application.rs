use super::super::super::json_utils::category_storage_key;
use super::super::super::sync_write::AttributeEventDraft;
use super::super::apply_attribute_to_state;
use super::builders::attribute_event_from_change;
use serde_json::Value;

pub(in crate::actions::character_ai) fn project_attribute_changes(
    mut state: Value,
    changes: &[Value],
) -> Result<Value, String> {
    for change in changes {
        apply_sync_change_to_state(&mut state, change)?;
    }
    Ok(state)
}

pub(in crate::actions::character_ai) fn apply_sync_change_to_state(
    state: &mut Value,
    change: &Value,
) -> Result<(), String> {
    let mut event = attribute_event_from_change(0, change)?;
    fill_event_old_value_and_apply(state, &mut event)
}

pub(in crate::actions::character_ai) fn fill_event_old_value_and_apply(
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
        _ => return Err(format!("未知属性事件 operation: {}", event.operation)),
    }
    apply_attribute_to_state(
        state,
        &event.category,
        &event.key,
        &event.operation,
        event.new_value.clone(),
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn attribute_value_in_state(state: &Value, category: &str, key: &str) -> Option<Value> {
    state
        .get(category_storage_key(category))
        .and_then(Value::as_object)
        .and_then(|attributes| attributes.get(key))
        .cloned()
}
