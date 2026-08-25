use super::super::config::{
    GENERATION_SPEC_REF_CONFIG_KEY, IMAGE_REF_CONFIG_KEY, PROMPT_ARTIFACT_REF_CONFIG_KEY,
    VIDEO_REF_CONFIG_KEY,
};
use super::detection::ResetTargetLookup;
use serde_json::Value;

#[cfg(test)]
pub(crate) fn clear_reset_generated_ref_payload(
    object: &mut serde_json::Map<String, Value>,
    targets: &crate::project::FreeformGeneratedResetTargets,
) {
    let lookup = ResetTargetLookup::from_targets(targets);
    clear_reset_generated_ref_payload_with_lookup(object, &lookup);
}

pub(crate) fn clear_reset_generated_ref_payload_with_lookup(
    object: &mut serde_json::Map<String, Value>,
    lookup: &ResetTargetLookup,
) {
    remove_config_ref(object, IMAGE_REF_CONFIG_KEY, &lookup.image_ids);
    remove_config_ref(object, VIDEO_REF_CONFIG_KEY, &lookup.video_ids);
    remove_config_ref(
        object,
        GENERATION_SPEC_REF_CONFIG_KEY,
        &lookup.generation_spec_ids,
    );
    remove_config_ref(
        object,
        PROMPT_ARTIFACT_REF_CONFIG_KEY,
        &lookup.prompt_artifact_ids,
    );
    remove_config_ref(object, "media_asset_id", &lookup.media_asset_ids);
}

fn remove_config_ref(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    ids: &std::collections::HashSet<i64>,
) {
    if object
        .get(key)
        .and_then(Value::as_i64)
        .is_some_and(|id| ids.contains(&id))
    {
        object.remove(key);
    }
}
