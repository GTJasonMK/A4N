use serde_json::Value;

use crate::rag::RagDataType;

use super::value::{first_string, value_as_string};
use super::{AnalysisRagItem, foreshadowing_entries};

pub fn analysis_rag_items(analysis: &Value) -> Vec<AnalysisRagItem> {
    let mut items = metadata_item(analysis).into_iter().collect::<Vec<_>>();
    items.extend(key_event_items(analysis));
    items.extend(foreshadowing_items(analysis));
    items
}

fn metadata_item(analysis: &Value) -> Option<AnalysisRagItem> {
    metadata_text(analysis).map(|text| AnalysisRagItem {
        id_suffix: "metadata".to_string(),
        data_type: RagDataType::ChapterMetadata,
        text,
    })
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
        if let Some(values) = metadata.get(key).and_then(Value::as_array) {
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
    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn key_event_items(analysis: &Value) -> impl Iterator<Item = AnalysisRagItem> + '_ {
    analysis
        .get("key_events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, event)| {
            first_string(event, &["description"]).map(|text| AnalysisRagItem {
                id_suffix: format!("event{}", index + 1),
                data_type: RagDataType::KeyEvent,
                text,
            })
        })
}

fn foreshadowing_items(analysis: &Value) -> impl Iterator<Item = AnalysisRagItem> {
    foreshadowing_entries(analysis)
        .into_iter()
        .enumerate()
        .map(|(index, entry)| AnalysisRagItem {
            id_suffix: format!("foreshadow{}", index + 1),
            data_type: RagDataType::Foreshadowing,
            text: entry.description,
        })
}
