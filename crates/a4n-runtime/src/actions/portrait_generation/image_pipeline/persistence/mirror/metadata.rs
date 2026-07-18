use serde_json::{Value, json};

use super::PortraitImageMirror;
use crate::actions::image_generation::normalize_image_metadata;

pub(super) fn portrait_mirror_metadata(
    input: &PortraitImageMirror<'_>,
    generated_at: &str,
) -> Option<Value> {
    normalize_image_metadata(Some(json!({
        "portrait_id": input.portrait_id,
        "profile_id": input.profile_id,
        "character_name": input.character_name,
        "portrait_type": input.portrait_type,
        "expression": input.expression,
        "pose": input.pose,
        "variant_kind": input.variant_kind,
        "auto_generated": input.auto_generated,
        "reference_image_ids": input.reference_image_ids,
        "image_type": "portrait",
        "generation": {
            "command": "mirror_portrait_to_generated_image",
            "image_type": "portrait",
            "provider_model": input.model,
            "width": input.width,
            "height": input.height,
            "prompt": input.prompt,
            "reference_image_ids": input.reference_image_ids,
            "generated_at": generated_at
        }
    })))
}
