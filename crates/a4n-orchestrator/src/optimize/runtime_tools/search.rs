use serde_json::{Value, json};

use super::specs::INFORMATION_SOURCES;

pub(super) fn search_context(
    context: &Value,
    source: Option<&str>,
    query: &str,
    limit: usize,
) -> Vec<Value> {
    let query_lower = query.to_lowercase();
    let mut out = Vec::new();
    match source {
        Some(source) => {
            if let Some(value) = context.get(source) {
                collect_matches(source, value, "$", &query_lower, limit, &mut out);
            }
        }
        None => {
            for source in INFORMATION_SOURCES {
                if let Some(value) = context.get(*source) {
                    collect_matches(source, value, "$", &query_lower, limit, &mut out);
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
    }
    out
}

pub(super) fn collect_matches(
    source: &str,
    value: &Value,
    path: &str,
    query_lower: &str,
    limit: usize,
    out: &mut Vec<Value>,
) {
    if out.len() >= limit {
        return;
    }
    match value {
        Value::String(text) if text.to_lowercase().contains(query_lower) => {
            out.push(json!({
                "source": source,
                "path": path,
                "snippet": search_snippet(text, query_lower),
            }));
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_matches(
                    source,
                    item,
                    &format!("{path}[{index}]"),
                    query_lower,
                    limit,
                    out,
                );
                if out.len() >= limit {
                    break;
                }
            }
        }
        Value::Object(object) => {
            for (key, child) in object {
                collect_matches(
                    source,
                    child,
                    &format!("{path}.{key}"),
                    query_lower,
                    limit,
                    out,
                );
                if out.len() >= limit {
                    break;
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

pub(super) fn search_snippet(text: &str, query_lower: &str) -> String {
    let lower = text.to_lowercase();
    let byte_index = lower.find(query_lower).unwrap_or(0);
    let char_index = lower[..byte_index].chars().count();
    let start = char_index.saturating_sub(60);
    let snippet = text.chars().skip(start).take(140).collect::<String>();
    snippet.trim().to_string()
}
