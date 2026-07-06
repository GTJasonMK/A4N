use std::collections::BTreeSet;

use a4n_core::error::AppError;
use a4n_core::rag::RagDataType;
use serde_json::{Value, json};

use super::{AnalysisRagItem, CharacterStateSnapshot, ForeshadowingEntry};
use crate::chapter::analysis::normalization::analysis_schema_error;

pub fn summary_text(analysis: &Value) -> Option<String> {
    let summaries = analysis.get("summaries")?;
    first_string(summaries, &["compressed", "one_line"]).or_else(|| {
        summaries
            .get("keywords")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(value_as_string)
                    .collect::<Vec<_>>()
                    .join("、")
            })
            .filter(|s| !s.trim().is_empty())
    })
}

pub(super) fn metadata_tags(analysis: &Value) -> Vec<String> {
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

pub fn character_state_snapshots(analysis: &Value) -> Vec<CharacterStateSnapshot> {
    let Some(states) = analysis.get("character_states").and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    states
        .iter()
        .filter_map(|(name, value)| {
            let obj = value.as_object()?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            let changes = obj
                .get("changes")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(value_as_string).collect::<Vec<_>>())?;
            let status = first_string(value, &["status"]);
            let state_summary = match (status.as_deref(), changes.is_empty()) {
                (Some(status), true) => status.to_string(),
                (Some(status), false) => format!("{status}；{}", changes.join("；")),
                (None, false) => changes.join("；"),
                (None, true) => return None,
            };
            if state_summary.trim().is_empty() {
                return None;
            }
            Some(CharacterStateSnapshot {
                name: name.to_string(),
                state_summary,
                key_attributes: json!({
                    "status": status,
                    "changes": changes,
                }),
                emotional_state: first_string(value, &["emotional_state"]),
                current_location: first_string(value, &["location"]),
                current_goal: None,
            })
        })
        .collect()
}

pub fn foreshadowing_entries(analysis: &Value) -> Vec<ForeshadowingEntry> {
    let Some(foreshadowing) = analysis.get("foreshadowing") else {
        return Vec::new();
    };
    let mut entries = Vec::new();

    if let Some(planted) = foreshadowing.get("planted").and_then(|v| v.as_array()) {
        for item in planted {
            if let Some(description) = first_string(item, &["description"]) {
                entries.push(ForeshadowingEntry {
                    description,
                    original_text: first_string(item, &["original_text"]),
                    category: first_string(item, &["category"]),
                    priority: first_string(item, &["priority"]),
                    related_entities: item.get("related_entities").cloned(),
                    status: "pending".to_string(),
                });
            }
        }
    }

    entries
}

pub fn analysis_rag_items(analysis: &Value) -> Vec<AnalysisRagItem> {
    let mut items = Vec::new();

    if let Some(metadata) = metadata_text(analysis) {
        items.push(AnalysisRagItem {
            id_suffix: "metadata".to_string(),
            data_type: RagDataType::ChapterMetadata.as_str().to_string(),
            content_kind: "metadata".to_string(),
            text: metadata,
            metadata: json!({}),
        });
    }

    for (idx, event) in analysis
        .get("key_events")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .enumerate()
    {
        if let Some(description) = first_string(event, &["description"]) {
            items.push(AnalysisRagItem {
                id_suffix: format!("event{}", idx + 1),
                data_type: RagDataType::KeyEvent.as_str().to_string(),
                content_kind: "key_event".to_string(),
                text: description,
                metadata: event.clone(),
            });
        }
    }

    for (idx, entry) in foreshadowing_entries(analysis).into_iter().enumerate() {
        items.push(AnalysisRagItem {
            id_suffix: format!("foreshadow{}", idx + 1),
            data_type: RagDataType::Foreshadowing.as_str().to_string(),
            content_kind: "foreshadowing".to_string(),
            text: entry.description,
            metadata: json!({
                "category": entry.category,
                "priority": entry.priority,
                "status": entry.status,
                "related_entities": entry.related_entities,
            }),
        });
    }

    items
}

pub fn priority_to_importance(priority: Option<&str>) -> Result<i32, AppError> {
    let priority = priority
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| analysis_schema_error("伏笔优先级缺失，不能写入重要度"))?;
    match priority.to_ascii_lowercase().as_str() {
        "high" | "重要" | "高" => Ok(5),
        "medium" | "中" => Ok(3),
        "low" | "低" => Ok(1),
        _ => Err(analysis_schema_error(format!("伏笔优先级非法: {priority}"))),
    }
}

fn metadata_text(analysis: &Value) -> Option<String> {
    let metadata = analysis.get("metadata")?;
    let mut parts = Vec::new();
    for (label, key) in [
        ("角色", "characters"),
        ("地点", "locations"),
        ("物品", "items"),
        ("标签", "tags"),
    ] {
        if let Some(values) = metadata.get(key).and_then(|v| v.as_array()) {
            let text = values
                .iter()
                .filter_map(value_as_string)
                .collect::<Vec<_>>()
                .join("、");
            if !text.is_empty() {
                parts.push(format!("{label}: {text}"));
            }
        }
    }
    for (label, key) in [("情绪基调", "tone"), ("时间线", "timeline_marker")] {
        if let Some(text) = first_string(metadata, &[key]) {
            parts.push(format!("{label}: {text}"));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .find_map(value_as_string)
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            let text = s.trim();
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }
        }
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub(super) fn merge_tags(existing: Option<&Value>, analysis_tags: Vec<String>) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(values) = existing.and_then(|v| v.as_array()) {
        tags.extend(values.iter().filter_map(value_as_string));
    }
    tags.extend(analysis_tags);
    dedup_strings(tags)
}

fn dedup_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            deduped.push(trimmed.to_string());
        }
    }
    deduped
}

pub(super) fn should_replace_summary(existing: Option<&str>, chapter_number: i32) -> bool {
    let Some(text) = existing.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    text.contains("摘要待生成")
        || text.contains("待分析")
        || text == format!("第{chapter_number}章")
        || (text.starts_with(&format!("第{chapter_number}章：")) && text.ends_with('字'))
}
