use serde_json::{Value, json};

use super::super::super::super::json::VideoPromptShot;

pub(super) fn optional_prompt_text(value: &str) -> Option<&str> {
    let text = value.trim();
    if text.is_empty() || matches!(text, "无" | "无特殊负面词" | "无特殊要求") {
        None
    } else {
        Some(text)
    }
}

pub(super) fn compact_shot_metadata(prompt: &VideoPromptShot) -> Value {
    json!({
        "shot_number": prompt.shot_number,
        "title": prompt.title.as_str(),
        "scene_summary": prompt.scene_summary.as_str(),
        "self_contained_context": prompt.self_contained_context.clone(),
        "start_state": prompt.start_state.clone(),
        "end_state": prompt.end_state.clone(),
        "transition_in": prompt.transition_in.clone()
    })
}
