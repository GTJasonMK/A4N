use super::super::character_discovery::character_identity_key;
use serde_json::Value;
use std::collections::BTreeSet;

pub(in crate::actions::character_ai) fn analysis_sync_character_names(
    analysis_character_names: &BTreeSet<String>,
    discovery_character_names: &BTreeSet<String>,
) -> BTreeSet<String> {
    analysis_character_names
        .iter()
        .chain(discovery_character_names.iter())
        .cloned()
        .collect()
}

pub(in crate::actions::character_ai) fn character_keys_from_names(
    names: &BTreeSet<String>,
) -> BTreeSet<String> {
    names
        .iter()
        .map(|name| character_identity_key(name))
        .filter(|key| !key.is_empty())
        .collect()
}

pub(in crate::actions::character_ai) fn character_names_from_chapter_analysis(
    analysis: &Value,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    if let Some(metadata) = analysis.get("metadata") {
        collect_character_name_values(metadata.get("characters"), &mut names);
    }

    if let Some(states) = analysis.get("character_states").and_then(Value::as_object) {
        for name in states.keys() {
            insert_analysis_character_name(name, &mut names);
        }
    }

    if let Some(events) = analysis.get("key_events").and_then(Value::as_array) {
        for event in events {
            collect_character_name_values(event.get("involved_characters"), &mut names);
        }
    }

    names
}

pub(in crate::actions::character_ai) fn character_names_from_discovery_result(
    result: &Value,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for field in ["created", "existing"] {
        let Some(items) = result.get(field).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            collect_character_name_values(item.get("name"), &mut names);
            collect_character_name_values(item.get("aliases"), &mut names);
            if let Some(base_attributes) = item.get("base_attributes") {
                collect_character_attribute_aliases(base_attributes, &mut names);
            }
            if let Some(current_state) = item.get("current_state") {
                collect_character_attribute_aliases(current_state, &mut names);
            }
        }
    }
    names
}

fn collect_character_attribute_aliases(attributes: &Value, names: &mut BTreeSet<String>) {
    if let Some(explicit) = attributes.get("explicit_attributes") {
        collect_character_name_values(explicit.get("aliases"), names);
    }
}

fn collect_character_name_values(value: Option<&Value>, names: &mut BTreeSet<String>) {
    match value {
        Some(Value::String(text)) => insert_analysis_character_name(text, names),
        Some(Value::Array(items)) => {
            for item in items {
                collect_character_name_values(Some(item), names);
            }
        }
        _ => {}
    }
}

fn insert_analysis_character_name(name: &str, names: &mut BTreeSet<String>) {
    let name = name.trim();
    if !name.is_empty() {
        names.insert(name.to_string());
    }
}
