use super::super::config::{
    GENERATION_SPEC_REF_CONFIG_KEYS, IMAGE_REF_CONFIG_KEYS, PROMPT_ARTIFACT_REF_CONFIG_KEYS,
    VIDEO_REF_CONFIG_KEYS, remove_config_any_i64_refs, remove_config_i64_array_refs_for_any,
};
use serde_json::Value;

pub fn clear_reset_generated_ref_payload(
    object: &mut serde_json::Map<String, Value>,
    targets: &crate::project::FreeformGeneratedResetTargets,
) {
    remove_config_any_i64_refs(object, IMAGE_REF_CONFIG_KEYS, &targets.image_ids);
    remove_config_i64_array_refs_for_any(object, "reference_image_ids", &targets.image_ids);
    remove_config_any_i64_refs(object, VIDEO_REF_CONFIG_KEYS, &targets.video_ids);
    remove_config_i64_array_refs_for_any(object, "video_ids", &targets.video_ids);
    remove_config_any_i64_refs(
        object,
        GENERATION_SPEC_REF_CONFIG_KEYS,
        &targets.generation_spec_ids,
    );
    remove_config_i64_array_refs_for_any(
        object,
        "generation_spec_ids",
        &targets.generation_spec_ids,
    );
    remove_config_any_i64_refs(
        object,
        PROMPT_ARTIFACT_REF_CONFIG_KEYS,
        &targets.prompt_artifact_ids,
    );
    remove_config_i64_array_refs_for_any(
        object,
        "prompt_artifact_ids",
        &targets.prompt_artifact_ids,
    );
    remove_config_any_i64_refs(object, &["media_asset_id"], &targets.media_asset_ids);
    remove_config_i64_array_refs_for_any(object, "media_asset_ids", &targets.media_asset_ids);
}
