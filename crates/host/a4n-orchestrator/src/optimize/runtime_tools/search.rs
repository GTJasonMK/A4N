use serde_json::{Value, json};

use a4n_api_contract::optimize::OPTIMIZATION_CONTEXT_SOURCES;

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
                collect_matches(source, value, "", &query_lower, limit, &mut out);
            }
        }
        None => {
            for source in OPTIMIZATION_CONTEXT_SOURCES {
                if let Some(value) = context.get(*source) {
                    collect_matches(source, value, "", &query_lower, limit, &mut out);
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
    }
    out
}

fn collect_matches(
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
                    &format!("{path}/{index}"),
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
                    &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
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

fn search_snippet(text: &str, query_lower: &str) -> String {
    let lower = text.to_lowercase();
    let byte_index = lower.find(query_lower).unwrap_or(0);
    // Case folding can expand one source character (for example İ → i + ◌̇).
    let char_index = text
        .chars()
        .scan(0, |offset, character| {
            *offset += character.to_lowercase().map(char::len_utf8).sum::<usize>();
            Some(*offset)
        })
        .take_while(|end| *end <= byte_index)
        .count();
    let start = char_index.saturating_sub(60);
    let snippet = text.chars().skip(start).take(140).collect::<String>();
    snippet.trim().to_string()
}
