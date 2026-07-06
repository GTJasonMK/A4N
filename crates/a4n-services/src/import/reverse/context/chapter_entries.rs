use std::collections::BTreeSet;

use a4n_core::models::chapter::{chapter, part_outline};
use serde_json::{Map, Value, json};

use crate::chapter::analysis::summary_text;
use crate::import::reverse_json::collect_json_strings;

fn text_preview(content: &str, max_chars: usize) -> String {
    let mut text = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() > max_chars {
        text = text.chars().take(max_chars).collect::<String>();
        text.push('…');
    }
    text
}

fn chapter_summary_text(ch: &chapter::Model) -> Option<String> {
    ch.analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            ch.summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| {
                    !summary.is_empty()
                        && !summary.starts_with(&format!("第{}章：", ch.chapter_number))
                })
                .map(str::to_string)
        })
}

fn analysis_metadata_array(analysis: Option<&Value>, key: &str) -> Option<Value> {
    analysis
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get(key))
        .filter(|v| v.as_array().is_some_and(|arr| !arr.is_empty()))
        .cloned()
}

fn analysis_value_path(analysis: Option<&Value>, path: &[&str]) -> Option<Value> {
    let mut current = analysis?;
    for key in path {
        current = current.get(*key)?;
    }
    if current.is_null() || current.as_array().is_some_and(|arr| arr.is_empty()) {
        None
    } else {
        Some(current.clone())
    }
}

fn chapter_title(title: Option<&str>) -> Option<String> {
    title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn chapter_outline_entry(ch: &chapter::Model) -> Value {
    let mut entry = Map::new();
    entry.insert("chapter_number".to_string(), json!(ch.chapter_number));
    add_optional_chapter_title(&mut entry, ch);
    add_summary_or_preview(&mut entry, ch, 240);
    Value::Object(entry)
}

pub(super) fn chapter_part_entry(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
        "theme": part.theme,
    })
}

pub(super) fn sample_chapter_entry(ch: &chapter::Model) -> Result<Value, String> {
    let mut entry = Map::new();
    entry.insert("chapter_number".to_string(), json!(ch.chapter_number));
    add_optional_chapter_title(&mut entry, ch);
    entry.insert(
        "content".to_string(),
        json!(text_preview(&ch.content, 6000)),
    );
    if let Some(analysis) = chapter_analysis_context(ch)? {
        entry.insert("analysis".to_string(), analysis);
    }
    Ok(Value::Object(entry))
}

fn chapter_analysis_context(ch: &chapter::Model) -> Result<Option<Value>, String> {
    let Some(analysis) = ch.analysis_data.as_ref() else {
        return Ok(None);
    };
    let analysis = analysis
        .as_object()
        .ok_or_else(|| format!("章节 #{} 的 analysis_data 必须是对象", ch.id))?;
    if analysis.is_empty() {
        return Ok(None);
    }
    let mut compact = Map::new();
    for key in [
        "metadata",
        "character_states",
        "foreshadowing",
        "key_events",
    ] {
        if let Some(value) = analysis.get(key).filter(|value| is_meaningful_json(value)) {
            compact.insert(key.to_string(), value.clone());
        }
    }
    if compact.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(compact)))
    }
}

fn is_meaningful_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

fn select_sample_indices(total: usize, limit: usize) -> BTreeSet<usize> {
    let mut indices = BTreeSet::new();
    if total == 0 || limit == 0 {
        return indices;
    }
    if total <= limit {
        indices.extend(0..total);
        return indices;
    }
    if limit == 1 {
        indices.insert(0);
        return indices;
    }
    for i in 0..limit {
        indices.insert(i * (total - 1) / (limit - 1));
    }
    indices
}

pub(super) fn sampled_chapters(chapters: &[chapter::Model], limit: usize) -> Vec<&chapter::Model> {
    let indices = select_sample_indices(chapters.len(), limit);
    chapters
        .iter()
        .enumerate()
        .filter_map(|(idx, chapter)| indices.contains(&idx).then_some(chapter))
        .collect()
}

pub(super) fn chapter_summary_for_reverse(ch: &chapter::Model) -> Value {
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

fn add_optional_chapter_title(entry: &mut Map<String, Value>, ch: &chapter::Model) {
    if let Some(title) = chapter_title(ch.title.as_deref()) {
        entry.insert("title".to_string(), json!(title));
    }
}

fn add_summary_or_preview(entry: &mut Map<String, Value>, ch: &chapter::Model, max_chars: usize) {
    if let Some(summary) = chapter_summary_text(ch) {
        entry.insert("summary".to_string(), json!(summary));
    } else {
        entry.insert(
            "content_preview".to_string(),
            json!(text_preview(&ch.content, max_chars)),
        );
    }
}
