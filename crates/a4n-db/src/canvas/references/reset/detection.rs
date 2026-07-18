use super::super::{
    MissingCanvasRef,
    config::{
        GENERATION_SPEC_REF_CONFIG_KEYS, IMAGE_REF_CONFIG_KEYS, PROMPT_ARTIFACT_REF_CONFIG_KEYS,
        VIDEO_REF_CONFIG_KEYS, first_config_array_i64_for_any, first_config_id_in,
    },
};
use a4n_core::models::canvas::canvas_node;

pub fn reset_missing_canvas_ref_for_node(
    node: &canvas_node::Model,
    targets: &crate::project::FreeformGeneratedResetTargets,
) -> Option<MissingCanvasRef> {
    if node.ref_type.as_deref() == Some("media_asset")
        && let Some(ref_id) = node.ref_id
        && targets.media_asset_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "media_asset",
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some("generation_spec")
        && let Some(ref_id) = node.ref_id
        && targets.generation_spec_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "generation_spec",
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some("prompt_artifact")
        && let Some(ref_id) = node.ref_id
        && targets.prompt_artifact_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "prompt_artifact",
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some("legacy_generated_image")
        && let Some(ref_id) = node.ref_id
        && targets.image_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "generated_image",
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some("legacy_generated_video")
        && let Some(ref_id) = node.ref_id
        && targets.video_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "generated_video",
            ref_id,
            clear_node_ref: true,
        });
    }
    if let Some(ref_id) = first_config_id_in(
        &node.config_json,
        IMAGE_REF_CONFIG_KEYS,
        &targets.image_ids,
    )
    .or_else(|| {
        first_config_array_i64_for_any(&node.config_json, "reference_image_ids", &targets.image_ids)
    }) {
        return Some(MissingCanvasRef {
            ref_type: "generated_image",
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) =
        first_config_id_in(&node.config_json, VIDEO_REF_CONFIG_KEYS, &targets.video_ids).or_else(
            || first_config_array_i64_for_any(&node.config_json, "video_ids", &targets.video_ids),
        )
    {
        return Some(MissingCanvasRef {
            ref_type: "generated_video",
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = first_config_id_in(
        &node.config_json,
        GENERATION_SPEC_REF_CONFIG_KEYS,
        &targets.generation_spec_ids,
    ) {
        return Some(MissingCanvasRef {
            ref_type: "generation_spec",
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = first_config_id_in(
        &node.config_json,
        PROMPT_ARTIFACT_REF_CONFIG_KEYS,
        &targets.prompt_artifact_ids,
    ) {
        return Some(MissingCanvasRef {
            ref_type: "prompt_artifact",
            ref_id,
            clear_node_ref: false,
        });
    }
    first_config_id_in(
        &node.config_json,
        &["media_asset_id"],
        &targets.media_asset_ids,
    )
    .map(|ref_id| MissingCanvasRef {
        ref_type: "media_asset",
        ref_id,
        clear_node_ref: false,
    })
}
