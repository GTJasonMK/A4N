use a4n_core::models::chapter::chapter;
use serde_json::{Map, Value, json};

use super::analysis::{analysis_metadata_array, analysis_value_path};
use super::formatting::{add_optional_chapter_title, add_summary_or_preview};
use a4n_core::import_reverse::json::collect_json_strings;

pub(in crate::import_reverse::context) fn chapter_summary_for_reverse(
    ch: &chapter::Model,
) -> Value {
    let analysis = ch.analysis_data.as_ref();
    let mut entry = Map::new();
    entry.insert("chapter_number".to_string(), json!(ch.chapter_number));
    add_optional_chapter_title(&mut entry, ch);
    add_summary_or_preview(&mut entry, ch, 240);

    if let Some(value) = analysis_metadata_array(analysis, "characters") {
        let key_characters = collect_json_strings(Some(&value), 6);
        if !key_characters.is_empty() {
            entry.insert("key_characters".to_string(), json!(key_characters));
        }
    }
    if let Some(value) = analysis_value_path(analysis, &["key_events"]) {
        let key_events = collect_json_strings(Some(&value), 3);
        if !key_events.is_empty() {
            entry.insert("key_events".to_string(), json!(key_events));
        }
    }

    Value::Object(entry)
}
