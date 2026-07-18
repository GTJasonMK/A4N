use crate::actions::image_generation::normalize_image_metadata;
use chrono::Utc;
use serde_json::{Value, json};

pub(super) fn imported_image_metadata(
    metadata_json: Option<Value>,
    source_file_name: String,
    source_extension: &'static str,
    mime_type: &'static str,
) -> Option<Value> {
    let mut metadata = match metadata_json {
        Some(Value::Object(map)) => Value::Object(map),
        Some(value) => json!({"metadata": value}),
        None => json!({}),
    };
    if let Some(map) = metadata.as_object_mut() {
        map.insert(
            "import".to_string(),
            json!({
                "external": true,
                "source_file_name": source_file_name,
                "source_extension": source_extension,
                "mime_type": mime_type,
                "imported_at": Utc::now().to_rfc3339(),
            }),
        );
    }
    normalize_image_metadata(Some(metadata))
}
