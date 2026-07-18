use serde_json::Value;

pub(crate) fn completed_provider_metadata(
    provider_metadata: Value,
    tail_frame_metadata: Option<Value>,
    tail_frame_capture_error: Option<String>,
) -> Option<Value> {
    if tail_frame_metadata.is_none() && tail_frame_capture_error.is_none() {
        return (!provider_metadata.is_null()).then_some(provider_metadata);
    }

    let mut metadata = serde_json::Map::new();
    if !provider_metadata.is_null() {
        metadata.insert("provider".to_string(), provider_metadata);
    }
    if let Some(tail_frame_metadata) = tail_frame_metadata {
        metadata.insert("tail_frame".to_string(), tail_frame_metadata);
    }
    if let Some(error) = tail_frame_capture_error {
        metadata.insert("tail_frame_capture_error".to_string(), Value::String(error));
    }
    Some(Value::Object(metadata))
}
