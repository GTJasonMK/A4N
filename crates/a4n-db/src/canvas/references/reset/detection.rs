use super::super::{
    MissingCanvasRef,
    config::{
        GENERATION_SPEC_REF_CONFIG_KEY, IMAGE_REF_CONFIG_KEY, PROMPT_ARTIFACT_REF_CONFIG_KEY,
        VIDEO_REF_CONFIG_KEY, config_id_in,
    },
};
use a4n_core::models::canvas::canvas_node;
use a4n_domain::CanvasNodeRefKind;

pub fn reset_missing_canvas_ref_for_node(
    node: &canvas_node::Model,
    targets: &crate::project::FreeformGeneratedResetTargets,
) -> Option<MissingCanvasRef> {
    if node.ref_type.as_deref() == Some(CanvasNodeRefKind::MediaAsset.as_str())
        && let Some(ref_id) = node.ref_id
        && targets.media_asset_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::MediaAsset,
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some(CanvasNodeRefKind::GenerationSpec.as_str())
        && let Some(ref_id) = node.ref_id
        && targets.generation_spec_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GenerationSpec,
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some(CanvasNodeRefKind::PromptArtifact.as_str())
        && let Some(ref_id) = node.ref_id
        && targets.prompt_artifact_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::PromptArtifact,
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some(CanvasNodeRefKind::GeneratedImage.as_str())
        && let Some(ref_id) = node.ref_id
        && targets.image_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GeneratedImage,
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some(CanvasNodeRefKind::GeneratedVideo.as_str())
        && let Some(ref_id) = node.ref_id
        && targets.video_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GeneratedVideo,
            ref_id,
            clear_node_ref: true,
        });
    }
    if let Some(ref_id) = config_id_in(&node.config_json, IMAGE_REF_CONFIG_KEY, &targets.image_ids)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GeneratedImage,
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = config_id_in(&node.config_json, VIDEO_REF_CONFIG_KEY, &targets.video_ids)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GeneratedVideo,
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = config_id_in(
        &node.config_json,
        GENERATION_SPEC_REF_CONFIG_KEY,
        &targets.generation_spec_ids,
    ) {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GenerationSpec,
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = config_id_in(
        &node.config_json,
        PROMPT_ARTIFACT_REF_CONFIG_KEY,
        &targets.prompt_artifact_ids,
    ) {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::PromptArtifact,
            ref_id,
            clear_node_ref: false,
        });
    }
    config_id_in(
        &node.config_json,
        "media_asset_id",
        &targets.media_asset_ids,
    )
    .map(|ref_id| MissingCanvasRef {
        ref_type: CanvasNodeRefKind::MediaAsset,
        ref_id,
        clear_node_ref: false,
    })
}
