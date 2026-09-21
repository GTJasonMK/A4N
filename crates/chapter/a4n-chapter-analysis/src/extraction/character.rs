use serde_json::{Value, json};

use super::CharacterStateSnapshot;
use super::value::{first_string, value_as_string};

pub fn character_state_snapshots(analysis: &Value) -> Vec<CharacterStateSnapshot> {
    let Some(states) = analysis.get("character_states").and_then(Value::as_object) else {
        return Vec::new();
    };

    states
        .iter()
        .filter_map(|(name, value)| character_state_snapshot(name, value))
        .collect()
}

fn character_state_snapshot(name: &str, value: &Value) -> Option<CharacterStateSnapshot> {
    let obj = value.as_object()?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let changes = obj
        .get("changes")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(value_as_string).collect::<Vec<_>>())?;
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
}
