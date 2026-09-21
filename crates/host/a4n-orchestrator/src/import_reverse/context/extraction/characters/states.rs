use serde_json::{Map, Value, json};

use crate::import_reverse::json::stringify_json_value;

use super::{CharacterMap, push_character_chapter};

pub(super) fn merge_character_states(
    map: &mut CharacterMap,
    analysis: &Value,
    chapter_number: i32,
    limit: usize,
) {
    let Some(states) = analysis.get("character_states").and_then(Value::as_object) else {
        return;
    };

    for (name, state) in states {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let state_hint = stringify_json_value(state, 240);
        if let Some(existing) = map.get_mut(name) {
            push_character_chapter(existing, chapter_number);
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
        push_character_chapter(&mut value, chapter_number);
        map.insert(name.to_string(), value);
    }
}
