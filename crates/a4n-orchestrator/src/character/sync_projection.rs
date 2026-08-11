use serde_json::Value;

use super::{
    apply_attribute_to_state, category_storage_key, required_attribute_change_operation,
    required_category, required_i32, required_string, required_usize, required_value,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterSyncAttributeEventDraft {
    pub chapter_number: i32,
    pub sequence: usize,
    pub stage: u8,
    pub category: String,
    pub key: String,
    pub operation: String,
    pub old_value: Value,
    pub new_value: Value,
    pub change_description: String,
    pub cause: String,
    pub evidence: String,
    pub from_classification: bool,
}

pub fn project_character_attribute_changes(
    mut state: Value,
    changes: &[Value],
) -> Result<Value, String> {
    for change in changes {
        apply_character_sync_change_to_state(&mut state, change)?;
    }
    Ok(state)
}

pub(super) fn apply_character_sync_change_to_state(
    state: &mut Value,
    change: &Value,
) -> Result<(), String> {
    let mut event = attribute_event_from_change(0, change)?;
    fill_event_old_value_and_apply(state, &mut event)
}

pub fn pending_character_sync_attribute_events(
    baseline_state: &Value,
    changes: &[Value],
    classification_suggestions: &[Value],
) -> Result<Vec<CharacterSyncAttributeEventDraft>, String> {
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
) -> Result<CharacterSyncAttributeEventDraft, String> {
    Ok(CharacterSyncAttributeEventDraft {
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
) -> Result<CharacterSyncAttributeEventDraft, String> {
    let reasoning = required_string(suggestion, "reasoning")?;
    let key = required_string(suggestion, "key")?;
    Ok(CharacterSyncAttributeEventDraft {
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
    event: &mut CharacterSyncAttributeEventDraft,
) -> Result<(), String> {
    let existing = attribute_value_in_state(state, &event.category, &event.key);
    match event.operation.as_str() {
        "add" => {
            if event.new_value.is_null() {
                return Err(format!(
                    "属性事件不一致：第 {} 章 add {}.{} 的 new_value 不能为空",
                    event.chapter_number, event.category, event.key
                ));
            }
            event.old_value = existing.unwrap_or(Value::Null);
        }
        "modify" => {
            if event.new_value.is_null() {
                return Err(format!(
                    "属性事件不一致：第 {} 章 modify {}.{} 的 new_value 不能为空",
                    event.chapter_number, event.category, event.key
                ));
            }
            event.old_value = existing.unwrap_or(Value::Null);
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
    .map_err(|err| err.to_string())
}

fn attribute_value_in_state(state: &Value, category: &str, key: &str) -> Option<Value> {
    state
        .get(category_storage_key(category))
        .and_then(Value::as_object)
        .and_then(|attributes| attributes.get(key))
        .cloned()
}
