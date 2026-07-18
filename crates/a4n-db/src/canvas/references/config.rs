use crate::canvas::CanvasDbError;
use serde_json::Value;

pub(super) const VIDEO_REF_CONFIG_KEYS: &[&str] = &[
    "generated_video_id",
    "legacy_generated_video_id",
    "source_video_id",
    "video_id",
];
pub(super) const IMAGE_REF_CONFIG_KEYS: &[&str] = &[
    "generated_image_id",
    "legacy_generated_image_id",
    "source_image_id",
    "first_frame_image_id",
];
pub(super) const GENERATION_SPEC_REF_CONFIG_KEYS: &[&str] = &["generation_spec_id"];
pub(super) const PROMPT_ARTIFACT_REF_CONFIG_KEYS: &[&str] = &["prompt_artifact_id"];

pub(super) fn config_contains_i64(config: &Value, keys: &[&str], target_id: i64) -> bool {
    keys.iter()
        .filter_map(|key| config.get(*key).and_then(Value::as_i64))
        .any(|id| id == target_id)
}

pub(super) fn config_contains_any_i64(config: &Value, keys: &[&str], ids: &[i64]) -> bool {
    keys.iter()
        .filter_map(|key| config.get(*key).and_then(Value::as_i64))
        .any(|id| ids.contains(&id))
}

pub(super) fn config_array_contains_i64(config: &Value, key: &str, target_id: i64) -> bool {
    config
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .filter_map(Value::as_i64)
                .any(|id| id == target_id)
        })
}

pub(super) fn config_array_contains_any_i64(config: &Value, key: &str, ids: &[i64]) -> bool {
    config
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .filter_map(Value::as_i64)
                .any(|id| ids.contains(&id))
        })
}

pub(super) fn first_config_i64(config: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| config.get(*key).and_then(Value::as_i64))
}

pub(super) fn first_config_id_in(config: &Value, keys: &[&str], ids: &[i64]) -> Option<i64> {
    keys.iter()
        .filter_map(|key| config.get(*key).and_then(Value::as_i64))
        .find(|id| ids.contains(id))
}

pub(super) fn first_config_array_i64_for_any(
    config: &Value,
    key: &str,
    ids: &[i64],
) -> Option<i64> {
    config
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .find(|id| ids.contains(id))
}

pub(super) fn first_config_array_i64(config: &Value, key: &str, target_id: i64) -> Option<i64> {
    config
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .find(|id| *id == target_id)
}

pub(super) fn remove_config_any_i64_refs(
    object: &mut serde_json::Map<String, Value>,
    keys: &[&str],
    ids: &[i64],
) {
    for key in keys {
        if object
            .get(*key)
            .and_then(Value::as_i64)
            .is_some_and(|id| ids.contains(&id))
        {
            object.remove(*key);
        }
    }
}

pub(super) fn remove_config_i64_refs(
    object: &mut serde_json::Map<String, Value>,
    keys: &[&str],
    target_id: i64,
) {
    for key in keys {
        if object
            .get(*key)
            .and_then(Value::as_i64)
            .is_some_and(|id| id == target_id)
        {
            object.remove(*key);
        }
    }
}

pub(super) fn remove_config_i64_array_refs_for_any(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    ids: &[i64],
) {
    let Some(values) = object.get(key).and_then(Value::as_array) else {
        return;
    };
    let retained = values
        .iter()
        .filter(|value| value.as_i64().is_none_or(|id| !ids.contains(&id)))
        .cloned()
        .collect::<Vec<_>>();
    if retained.is_empty() {
        object.remove(key);
    } else {
        object.insert(key.to_string(), Value::Array(retained));
    }
}

pub(super) fn remove_config_i64_array_refs(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    target_id: i64,
) {
    let Some(values) = object.get(key).and_then(Value::as_array) else {
        return;
    };
    let retained = values
        .iter()
        .filter(|value| value.as_i64() != Some(target_id))
        .cloned()
        .collect::<Vec<_>>();
    if retained.is_empty() {
        object.remove(key);
    } else {
        object.insert(key.to_string(), Value::Array(retained));
    }
}

pub(super) fn validate_positive_id(value: i64, label: &str) -> Result<(), CanvasDbError> {
    if value > 0 {
        Ok(())
    } else {
        Err(CanvasDbError::InvalidArgument(format!(
            "{label} 必须为正数"
        )))
    }
}
