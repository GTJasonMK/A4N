use super::super::super::json_utils::{
    required_attribute_change_operation, required_category, required_i32, required_string,
    required_usize, required_value,
};
use super::super::super::sync_write::AttributeEventDraft;
use super::application::fill_event_old_value_and_apply;
use serde_json::Value;

pub(in crate::actions::character_ai) fn pending_attribute_events(
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

pub(in crate::actions::character_ai) fn attribute_event_from_change(
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
