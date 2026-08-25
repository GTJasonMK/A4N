use serde_json::{Map, Value, json};

use crate::import_reverse::json::optional_string;

use super::{CharacterMap, normalize_character_record_name, push_character_chapter};

pub(super) fn merge_metadata_characters(
    map: &mut CharacterMap,
    analysis: &Value,
    chapter_number: i32,
    limit: usize,
) {
    let Some(characters) = analysis
        .get("metadata")
        .and_then(|metadata| metadata.get("characters"))
        .and_then(Value::as_array)
    else {
        return;
    };

    for item in characters {
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
            push_character_chapter(existing, chapter_number);
            continue;
        }
        if map.len() >= limit {
            continue;
        }
        push_character_chapter(&mut value, chapter_number);
        map.insert(name, value);
    }
}
