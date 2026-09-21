use serde_json::Value;

use super::super::value::{dedup_strings, first_string, value_as_string};

pub fn metadata_tags(analysis: &Value) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(metadata) = analysis.get("metadata") {
        if let Some(tone) = first_string(metadata, &["tone"]) {
            tags.push(tone);
        }
        if let Some(items) = metadata.get("tags").and_then(|v| v.as_array()) {
            tags.extend(items.iter().filter_map(value_as_string));
        }
    }
    dedup_strings(tags)
}
