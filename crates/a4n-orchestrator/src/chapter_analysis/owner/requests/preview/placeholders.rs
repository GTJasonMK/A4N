use serde_json::{Value, json};

pub(super) fn facts_preview_placeholder() -> Value {
    json!({
        "characters": [],
        "locations": [],
        "items": [],
        "event_candidates": [],
        "foreshadowing_candidates": [],
        "preview_note": "预览阶段尚未调用事实抽取；运行时会使用上一阶段真实 JSON"
    })
}
