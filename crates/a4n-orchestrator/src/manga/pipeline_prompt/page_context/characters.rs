use std::collections::HashSet;

use serde_json::{Map, Value, json};

pub(in crate::manga::pipeline_prompt) fn character_names_for_page(
    page_plan: &Value,
    events: &[Value],
) -> HashSet<String> {
    let mut names = HashSet::new();
    for panel in page_plan
        .get("panels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        collect_strings(panel.get("participants"), &mut names);
    }
    for event in events {
        collect_strings(event.get("participants"), &mut names);
    }
    names
}

pub(in crate::manga::pipeline_prompt) fn filter_characters(
    value: Option<&Value>,
    names: &HashSet<String>,
) -> Value {
    let Some(object) = value.and_then(Value::as_object) else {
        return json!({});
    };
    if names.is_empty() {
        return Value::Object(object.clone());
    }
    Value::Object(
        object
            .iter()
            .filter(|(key, value)| {
                names.contains(key.as_str())
                    || value
                        .get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|name| names.contains(name.trim()))
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Map<_, _>>(),
    )
}

pub(in crate::manga::pipeline_prompt) fn filter_reference_assets(
    value: Option<&Value>,
    names: &HashSet<String>,
) -> Value {
    let Some(object) = value.and_then(Value::as_object) else {
        return Value::Null;
    };
    if names.is_empty() {
        return Value::Object(object.clone());
    }

    let portraits = object
        .get("character_portraits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    item.get("character_name")
                        .and_then(Value::as_str)
                        .is_some_and(|name| names.contains(name.trim()))
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let summary = portraits
        .iter()
        .filter_map(|item| {
            let name = item.get("character_name")?.as_str()?.trim();
            let prompt = item
                .get("prompt")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if prompt.is_empty() {
                Some(name.to_string())
            } else {
                Some(format!("{name}: {prompt}"))
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    json!({
        "character_portraits": portraits,
        "character_portrait_summary": summary,
    })
}

fn collect_strings(value: Option<&Value>, target: &mut HashSet<String>) {
    match value {
        Some(Value::String(text)) => {
            let text = text.trim();
            if !text.is_empty() {
                target.insert(text.to_string());
            }
        }
        Some(Value::Array(items)) => {
            for item in items {
                collect_strings(Some(item), target);
            }
        }
        _ => {}
    }
}
