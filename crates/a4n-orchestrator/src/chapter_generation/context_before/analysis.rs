use super::shared::chapter_context_label;
use a4n_core::chapter_analysis::{character_state_snapshots, foreshadowing_entries};
use a4n_core::models::chapter::chapter;

pub(super) fn chapter_analysis_context_line(chapter: &chapter::Model) -> Option<String> {
    let analysis = chapter.analysis_data.as_ref()?;
    let mut parts = Vec::new();

    if let Some(metadata) = analysis.get("metadata").and_then(|v| v.as_object()) {
        for (label, key, limit) in [
            ("角色", "characters", 6usize),
            ("地点", "locations", 4usize),
            ("物品", "items", 4usize),
            ("标签", "tags", 4usize),
        ] {
            let values = json_string_list(metadata.get(key), limit);
            if !values.is_empty() {
                parts.push(format!("{label}: {}", values.join("、")));
            }
        }
        if let Some(tone) = metadata.get("tone").and_then(json_string) {
            parts.push(format!("基调: {tone}"));
        }
    }

    let events = analysis
        .get("key_events")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|event| event.get("description").and_then(json_string))
        .take(3)
        .collect::<Vec<_>>();
    if !events.is_empty() {
        parts.push(format!("关键事件: {}", events.join("；")));
    }

    let states = character_state_snapshots(analysis)
        .into_iter()
        .take(4)
        .map(|snapshot| format!("{}: {}", snapshot.name, snapshot.state_summary))
        .collect::<Vec<_>>();
    if !states.is_empty() {
        parts.push(format!("角色状态: {}", states.join("；")));
    }

    let foreshadowing = foreshadowing_entries(analysis)
        .into_iter()
        .take(3)
        .map(|entry| entry.description)
        .collect::<Vec<_>>();
    if !foreshadowing.is_empty() {
        parts.push(format!("伏笔/悬念: {}", foreshadowing.join("；")));
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!(
            "{}：{}",
            chapter_context_label(chapter.chapter_number, chapter.title.as_deref()),
            parts.join(" | ")
        ))
    }
}

fn json_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn json_string_list(value: Option<&serde_json::Value>, limit: usize) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(json_string)
        .take(limit)
        .collect()
}
