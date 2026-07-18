use a4n_core::models::chapter::chapter;
use serde_json::{Value, json};

fn push_foreshadowing_entry(out: &mut Vec<Value>, chapter_number: i32, item: &Value, status: &str) {
    match item {
        Value::String(text) => {
            let text = text.trim();
            if !text.is_empty() {
                out.push(json!({"chapter": chapter_number, "content": text, "status": status}));
            }
        }
        Value::Object(obj) => {
            let mut entry = obj.clone();
            entry.insert("chapter".to_string(), json!(chapter_number));
            entry
                .entry("status".to_string())
                .or_insert_with(|| json!(status));
            out.push(Value::Object(entry));
        }
        _ => {}
    }
}

pub(in crate::import_reverse::context) fn collect_analysis_foreshadowing(
    chapters: &[chapter::Model],
    limit: usize,
) -> Vec<Value> {
    let mut out = Vec::new();
    for ch in chapters {
        let Some(foreshadowing) = ch
            .analysis_data
            .as_ref()
            .and_then(|analysis| analysis.get("foreshadowing"))
        else {
            continue;
        };
        for (key, status) in [
            ("planted", "pending"),
            ("resolved", "resolved"),
            ("tensions", "tension"),
        ] {
            let Some(items) = foreshadowing.get(key).and_then(Value::as_array) else {
                continue;
            };
            for item in items {
                push_foreshadowing_entry(&mut out, ch.chapter_number, item, status);
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out
}
