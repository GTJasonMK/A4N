use serde_json::Value;

use super::super::value::{dedup_strings, value_as_string};

pub fn merge_tags(existing: Option<&Value>, analysis_tags: Vec<String>) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(values) = existing.and_then(|v| v.as_array()) {
        tags.extend(values.iter().filter_map(value_as_string));
    }
    tags.extend(analysis_tags);
    dedup_strings(tags)
}
