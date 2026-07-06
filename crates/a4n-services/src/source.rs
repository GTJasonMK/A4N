//! Shared source metadata helpers for chapter-derived artifacts.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[cfg(test)]
use a4n_core::error::AppError;

#[derive(Debug, Clone, Copy)]
pub struct ChapterSourceContext<'a> {
    pub content: &'a str,
}

pub fn source_content_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn source_metadata(source: ChapterSourceContext<'_>) -> Value {
    json!({
        "content_sha256": source_content_sha256(source.content),
        "content_chars": source.content.chars().count(),
    })
}

pub(crate) fn source_metadata_with_flags(
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
        .and_then(|m| m.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
}

pub fn source_metadata_matches(stored: &Value, current: &Value) -> bool {
    let stored_hash = stored
        .get("content_sha256")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());
    let current_hash = current
        .get("content_sha256")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());
    stored_hash == current_hash
}

#[cfg(test)]
pub(crate) fn artifact_matches_source(
    layout: &Value,
    source_metadata: &Value,
) -> Result<bool, AppError> {
    let stored = layout
        .get("source")
        .ok_or_else(|| AppError::InvalidArgument("产物 layout 缺少 source 元数据".into()))?;
    Ok(source_metadata_matches(stored, source_metadata))
}

pub(crate) fn with_source_metadata(
    layout: Option<Value>,
    source: ChapterSourceContext<'_>,
) -> Option<Value> {
    let source_metadata = source_metadata(source);
    match layout {
        Some(Value::Object(mut map)) => {
            map.insert("source".into(), source_metadata);
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "value": value,
            "source": source_metadata,
        })),
        None => Some(json!({
            "source": source_metadata,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_match_requires_source_metadata() {
        let current = source_metadata(ChapterSourceContext {
            content: "current chapter",
        });

        let err = artifact_matches_source(&json!({"checkpoint_data": {}}), &current).unwrap_err();

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("source 元数据"))
        );
    }

    #[test]
    fn artifact_match_compares_source_hash() {
        let stored = source_metadata(ChapterSourceContext {
            content: "current chapter",
        });
        let matching_layout = json!({ "source": stored });
        let changed_content = source_metadata(ChapterSourceContext {
            content: "changed chapter",
        });

        assert!(artifact_matches_source(&matching_layout, &stored).unwrap());
        assert!(!artifact_matches_source(&matching_layout, &changed_content).unwrap());
    }
}
