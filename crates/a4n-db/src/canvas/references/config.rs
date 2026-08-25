use crate::canvas::CanvasDbError;
use serde_json::Value;

pub(super) const VIDEO_REF_CONFIG_KEY: &str = "generated_video_id";
pub(super) const IMAGE_REF_CONFIG_KEY: &str = "generated_image_id";
pub(super) const GENERATION_SPEC_REF_CONFIG_KEY: &str = "generation_spec_id";
pub(super) const PROMPT_ARTIFACT_REF_CONFIG_KEY: &str = "prompt_artifact_id";

pub(super) fn config_id_in(
    config: &Value,
    key: &str,
    contains: impl Fn(&i64) -> bool,
) -> Option<i64> {
    config.get(key).and_then(Value::as_i64).filter(contains)
}

pub(super) fn remove_config_any_i64_ref(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    contains: impl Fn(&i64) -> bool,
) {
    if object
        .get(key)
        .and_then(Value::as_i64)
        .is_some_and(|id| contains(&id))
    {
        object.remove(key);
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
