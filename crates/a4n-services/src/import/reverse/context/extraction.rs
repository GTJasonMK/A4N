use std::collections::BTreeMap;

use a4n_core::models::chapter::chapter;
use serde_json::{Map, Value, json};

use crate::import::reverse_json::{optional_string, stringify_json_value};

pub(super) fn collect_extracted_characters(
    chapters: &[chapter::Model],
    limit: usize,
) -> Vec<Value> {
    let mut map = BTreeMap::<String, Map<String, Value>>::new();

    for ch in chapters {
        let Some(analysis) = ch.analysis_data.as_ref() else {
            continue;
        };
        if let Some(chars) = analysis
            .get("metadata")
            .and_then(|metadata| metadata.get("characters"))
            .and_then(Value::as_array)
        {
            for item in chars {
                let (name, mut value) = match item {
                    Value::String(name) => {
                        let name = name.trim();
                        if name.is_empty() {
                            continue;
                        }
                        let mut value = Map::new();
                        value.insert("name".to_string(), json!(name));
                        (name.to_string(), value)
                    }
                    Value::Object(obj) => {
                        let Some(name) = optional_string(item, "name") else {
                            continue;
                        };
                        (name, obj.clone())
                    }
                    _ => continue,
                };
                normalize_character_record_name(&mut value, &name);
                if let Some(existing) = map.get_mut(&name) {
                    push_character_chapter(existing, ch.chapter_number);
                    continue;
                }
                if map.len() >= limit {
                    continue;
                }
                push_character_chapter(&mut value, ch.chapter_number);
                map.insert(name, value);
            }
        }

        if let Some(states) = analysis.get("character_states").and_then(Value::as_object) {
            for (name, state) in states {
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                let state_hint = stringify_json_value(state, 240);
                if let Some(existing) = map.get_mut(name) {
                    push_character_chapter(existing, ch.chapter_number);
                    existing
                        .entry("state_hint".to_string())
                        .or_insert_with(|| json!(state_hint));
                    continue;
                }
                if map.len() >= limit {
                    continue;
                }
                let mut value = Map::new();
                value.insert("name".to_string(), json!(name));
                value.insert("state_hint".to_string(), json!(state_hint));
                push_character_chapter(&mut value, ch.chapter_number);
                map.insert(name.to_string(), value);
            }
        }
    }

    map.into_values().map(Value::Object).collect()
}

fn normalize_character_record_name(value: &mut Map<String, Value>, name: &str) {
    value.insert("name".to_string(), json!(name));
}

fn push_character_chapter(value: &mut Map<String, Value>, chapter_number: i32) {
    if !value.get("chapters").is_some_and(Value::is_array) {
        value.insert("chapters".to_string(), json!([]));
    }
    let Some(chapters) = value.get_mut("chapters").and_then(Value::as_array_mut) else {
        return;
    };
    if !chapters
        .iter()
        .any(|value| value.as_i64() == Some(chapter_number as i64))
    {
        chapters.push(json!(chapter_number));
    }
}

pub(super) fn dedup_character_records(values: Vec<Value>) -> Vec<Value> {
    let mut map = BTreeMap::<String, Value>::new();
    for value in values {
        let Some(name) = value
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .map(str::to_string)
        else {
            continue;
        };
        if name.is_empty() || map.contains_key(&name) {
            continue;
        }
        let mut value = value;
        if let Some(obj) = value.as_object_mut() {
            normalize_character_record_name(obj, &name);
        }
        map.insert(name, value);
    }
    map.into_values().collect()
}

fn push_foreshadowing_entry(out: &mut Vec<Value>, chapter_number: i32, item: &Value, status: &str) {
    match item {
        Value::String(text) => {
            let text = text.trim();
            if !text.is_empty() {
                out.push(json!({
                    "chapter": chapter_number,
                    "content": text,
                    "status": status,
                }));
            }
        }
        Value::Object(obj) => {
            let mut entry = obj.clone();
            entry.insert("chapter".to_string(), json!(chapter_number));
            entry
                .entry("status".to_string())
                .or_insert_with(|| json!(status));
            out.push(Value::Object(entry));
        }
        _ => {}
    }
}

pub(super) fn collect_analysis_foreshadowing(
    chapters: &[chapter::Model],
    limit: usize,
) -> Vec<Value> {
    let mut out = Vec::new();
    for ch in chapters {
        let Some(foreshadowing) = ch
            .analysis_data
            .as_ref()
            .and_then(|analysis| analysis.get("foreshadowing"))
        else {
            continue;
        };
        for (key, status) in [
            ("planted", "pending"),
            ("resolved", "resolved"),
            ("tensions", "tension"),
        ] {
            let Some(items) = foreshadowing.get(key).and_then(Value::as_array) else {
                continue;
            };
            for item in items {
                push_foreshadowing_entry(&mut out, ch.chapter_number, item, status);
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out
}
