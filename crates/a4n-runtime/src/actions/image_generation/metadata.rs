use super::*;

pub(super) fn merge_image_metadata(
    metadata_json: Option<Value>,
    source_metadata: Option<Value>,
) -> Option<Value> {
    merge_source_metadata(metadata_json, source_metadata, false)
}

pub(super) fn merge_direct_image_source_metadata(
    metadata_json: Option<Value>,
    source_metadata: Option<Value>,
) -> Option<Value> {
    merge_source_metadata(metadata_json, source_metadata, true)
}

pub(super) fn merge_source_metadata(
    metadata_json: Option<Value>,
    source_metadata: Option<Value>,
    preserve_existing_source: bool,
) -> Option<Value> {
    let Some(source_metadata) = source_metadata else {
        return metadata_json;
    };

    match metadata_json {
        Some(Value::Object(mut map)) => {
            let key = if preserve_existing_source && map.contains_key("source") {
                "chapter_source"
            } else {
                "source"
            };
            map.insert(key.into(), source_metadata);
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

pub(super) fn merge_direct_image_metadata(
    base: Option<Value>,
    extra: Option<Value>,
) -> Option<Value> {
    match (base, extra) {
        (Some(Value::Object(mut base)), Some(Value::Object(extra))) => {
            base.extend(extra);
            Some(Value::Object(base))
        }
        (Some(Value::Object(mut base)), Some(extra)) => {
            base.insert("extra".to_string(), extra);
            Some(Value::Object(base))
        }
        (Some(base), Some(Value::Object(mut extra))) => {
            extra.insert("base".to_string(), base);
            Some(Value::Object(extra))
        }
        (Some(base), Some(extra)) => Some(json!({
            "base": base,
            "extra": extra,
        })),
        (Some(base), None) => Some(base),
        (None, Some(extra)) => Some(extra),
        (None, None) => None,
    }
}
