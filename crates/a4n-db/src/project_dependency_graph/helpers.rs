use super::*;

pub(super) fn count_strings<'a>(values: impl Iterator<Item = &'a str>) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for value in values {
        count_string_value(&mut counts, value);
    }
    counts
}

pub(super) fn count_string_value(counts: &mut BTreeMap<String, u64>, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    *counts.entry(value.to_string()).or_insert(0) += 1;
}

pub(super) fn prompt_type_in(row: &chapter_video_prompt::Model, prompt_types: &[&str]) -> bool {
    prompt_type_str_in(&row.prompt_type, prompt_types)
}

pub(super) fn prompt_type_str_in(prompt_type: &str, prompt_types: &[&str]) -> bool {
    prompt_types.contains(&prompt_type)
}

pub(super) fn first_available_graph_source<'a>(sources: &[(&'a str, u64)]) -> Option<&'a str> {
    sources
        .iter()
        .find_map(|(node_id, count)| (*count > 0).then_some(*node_id))
}

pub(super) fn outline_range_summary(start: Option<i32>, end: Option<i32>) -> Option<String> {
    match (start, end) {
        (Some(start), Some(end)) => Some(format!("覆盖第 {start} 到第 {end} 章。")),
        _ => None,
    }
}

pub(super) fn video_frame_image_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    let metadata = metadata?;
    metadata
        .get("generation_context")
        .and_then(Value::as_object)
        .and_then(|context| context.get("prompt_id"))
        .and_then(Value::as_i64)
        .or_else(|| metadata.get("prompt_id").and_then(Value::as_i64))
        .filter(|id| *id > 0)
}

// tests stay in services for now
// #[cfg(test)]
// mod tests;
