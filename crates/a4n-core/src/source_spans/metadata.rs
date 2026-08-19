use serde_json::{Value, json};

use super::source_text_hash;

#[derive(Debug, Clone, Copy)]
pub struct ChapterSourceContext<'a> {
    pub content: &'a str,
}

pub fn source_metadata(source: ChapterSourceContext<'_>) -> Value {
    json!({
        "content_sha256": source_text_hash(source.content),
        "content_chars": source.content.chars().count(),
    })
}

pub fn source_metadata_with_flags(
    source: ChapterSourceContext<'_>,
    flags: impl IntoIterator<Item = (&'static str, bool)>,
) -> Value {
    let mut metadata = source_metadata(source);
    if let Value::Object(map) = &mut metadata {
        for (key, value) in flags {
            map.insert(key.into(), json!(value));
        }
    }
    metadata
}

pub fn metadata_source_content_sha256(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|value| value.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests;
