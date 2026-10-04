use a4n_domain::chapter::ChapterRecord;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

use self::metadata::merge_metadata_characters;
use self::states::merge_character_states;

mod metadata;
mod states;

type CharacterMap = BTreeMap<String, Map<String, Value>>;

pub(in crate::import_reverse::context) fn collect_extracted_characters(
    chapters: &[ChapterRecord],
    limit: usize,
) -> Vec<Value> {
    let mut map = CharacterMap::new();

    for ch in chapters {
        let Some(analysis) = ch.analysis_data.as_ref() else {
            continue;
        };
        merge_metadata_characters(&mut map, analysis, ch.chapter_number, limit);
        merge_character_states(&mut map, analysis, ch.chapter_number, limit);
    }

    map.into_values().map(Value::Object).collect()
}

pub(super) fn normalize_character_record_name(value: &mut Map<String, Value>, name: &str) {
    value.insert("name".to_string(), json!(name));
}

pub(super) fn push_character_chapter(value: &mut Map<String, Value>, chapter_number: i32) {
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

pub(in crate::import_reverse::context) fn dedup_character_records(
    values: Vec<Value>,
) -> Vec<Value> {
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
