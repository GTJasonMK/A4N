use super::super::config::{
    GENERATION_SPEC_REF_CONFIG_KEY, IMAGE_REF_CONFIG_KEY, PROMPT_ARTIFACT_REF_CONFIG_KEY,
    VIDEO_REF_CONFIG_KEY, remove_config_any_i64_ref,
};
use serde_json::Value;

pub fn clear_reset_generated_ref_payload(
    object: &mut serde_json::Map<String, Value>,
    targets: &crate::project::FreeformGeneratedResetTargets,
) {
    remove_config_any_i64_ref(object, IMAGE_REF_CONFIG_KEY, &targets.image_ids);
    remove_config_any_i64_ref(object, VIDEO_REF_CONFIG_KEY, &targets.video_ids);
    remove_config_any_i64_ref(
        object,
        GENERATION_SPEC_REF_CONFIG_KEY,
        &targets.generation_spec_ids,
    );
    remove_config_any_i64_ref(
        object,
        PROMPT_ARTIFACT_REF_CONFIG_KEY,
        &targets.prompt_artifact_ids,
    );
    remove_config_any_i64_ref(object, "media_asset_id", &targets.media_asset_ids);
}
