const CANVAS_TRANSFER_RUNTIME_IDENTITY_KEYS: [&str; 3] =
    ["workflow_run_id", "provider_job_id", "config_id"];

pub fn clear_canvas_transfer_runtime_identity(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for key in CANVAS_TRANSFER_RUNTIME_IDENTITY_KEYS {
                object.remove(key);
            }
            for value in object.values_mut() {
                clear_canvas_transfer_runtime_identity(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                clear_canvas_transfer_runtime_identity(value);
            }
        }
        _ => {}
    }
}

pub fn canvas_transfer_runtime_identity_key(value: &serde_json::Value) -> Option<&'static str> {
    match value {
        serde_json::Value::Object(object) => CANVAS_TRANSFER_RUNTIME_IDENTITY_KEYS
            .into_iter()
            .find(|key| object.contains_key(*key))
            .or_else(|| {
                object
                    .values()
                    .find_map(canvas_transfer_runtime_identity_key)
            }),
        serde_json::Value::Array(values) => {
            values.iter().find_map(canvas_transfer_runtime_identity_key)
        }
        _ => None,
    }
}
