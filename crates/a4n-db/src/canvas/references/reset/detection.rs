use super::super::{
    MissingCanvasRef,
    config::{
        GENERATION_SPEC_REF_CONFIG_KEY, IMAGE_REF_CONFIG_KEY, PROMPT_ARTIFACT_REF_CONFIG_KEY,
        VIDEO_REF_CONFIG_KEY,
    },
};
#[cfg(test)]
use a4n_core::models::canvas::canvas_node;
use a4n_domain::CanvasNodeRefKind;
use std::collections::HashSet;

pub(crate) struct ResetTargetLookup {
    pub(crate) image_ids: HashSet<i64>,
    pub(crate) video_ids: HashSet<i64>,
    pub(crate) media_asset_ids: HashSet<i64>,
    pub(crate) generation_spec_ids: HashSet<i64>,
    pub(crate) prompt_artifact_ids: HashSet<i64>,
}

impl ResetTargetLookup {
    pub(crate) fn from_targets(targets: &crate::project::FreeformGeneratedResetTargets) -> Self {
        Self {
            image_ids: targets.image_ids.iter().copied().collect(),
            video_ids: targets.video_ids.iter().copied().collect(),
            media_asset_ids: targets.media_asset_ids.iter().copied().collect(),
            generation_spec_ids: targets.generation_spec_ids.iter().copied().collect(),
            prompt_artifact_ids: targets.prompt_artifact_ids.iter().copied().collect(),
        }
    }
}

#[cfg(test)]
pub(crate) fn reset_missing_canvas_ref_for_node(
    node: &canvas_node::Model,
    targets: &crate::project::FreeformGeneratedResetTargets,
) -> Option<MissingCanvasRef> {
    reset_missing_canvas_ref_for_fields(
        node.ref_type.as_deref(),
        node.ref_id,
        &node.config_json,
        targets,
    )
}

#[cfg(test)]
pub(in crate::canvas::references) fn reset_missing_canvas_ref_for_fields(
    ref_type: Option<&str>,
    ref_id: Option<i64>,
    config_json: &serde_json::Value,
    targets: &crate::project::FreeformGeneratedResetTargets,
) -> Option<MissingCanvasRef> {
    let lookup = ResetTargetLookup::from_targets(targets);
    reset_missing_canvas_ref_for_fields_with_lookup(ref_type, ref_id, config_json, &lookup)
}

pub(crate) fn reset_missing_canvas_ref_for_fields_with_lookup(
    ref_type: Option<&str>,
    ref_id: Option<i64>,
    config_json: &serde_json::Value,
    lookup: &ResetTargetLookup,
) -> Option<MissingCanvasRef> {
    if ref_type == Some(CanvasNodeRefKind::MediaAsset.as_str())
        && let Some(ref_id) = ref_id
        && lookup.media_asset_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::MediaAsset,
            ref_id,
            clear_node_ref: true,
        });
    }
    if ref_type == Some(CanvasNodeRefKind::GenerationSpec.as_str())
        && let Some(ref_id) = ref_id
        && lookup.generation_spec_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GenerationSpec,
            ref_id,
            clear_node_ref: true,
        });
    }
    if ref_type == Some(CanvasNodeRefKind::PromptArtifact.as_str())
        && let Some(ref_id) = ref_id
        && lookup.prompt_artifact_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::PromptArtifact,
            ref_id,
            clear_node_ref: true,
        });
    }
    if ref_type == Some(CanvasNodeRefKind::GeneratedImage.as_str())
        && let Some(ref_id) = ref_id
        && lookup.image_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GeneratedImage,
            ref_id,
            clear_node_ref: true,
        });
    }
    if ref_type == Some(CanvasNodeRefKind::GeneratedVideo.as_str())
        && let Some(ref_id) = ref_id
        && lookup.video_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GeneratedVideo,
            ref_id,
            clear_node_ref: true,
        });
    }
    if let Some(ref_id) = config_id_in_lookup(config_json, IMAGE_REF_CONFIG_KEY, &lookup.image_ids)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GeneratedImage,
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = config_id_in_lookup(config_json, VIDEO_REF_CONFIG_KEY, &lookup.video_ids)
    {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GeneratedVideo,
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = config_id_in_lookup(
        config_json,
        GENERATION_SPEC_REF_CONFIG_KEY,
        &lookup.generation_spec_ids,
    ) {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::GenerationSpec,
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = config_id_in_lookup(
        config_json,
        PROMPT_ARTIFACT_REF_CONFIG_KEY,
        &lookup.prompt_artifact_ids,
    ) {
        return Some(MissingCanvasRef {
            ref_type: CanvasNodeRefKind::PromptArtifact,
            ref_id,
            clear_node_ref: false,
        });
    }
    config_id_in_lookup(config_json, "media_asset_id", &lookup.media_asset_ids).map(|ref_id| {
        MissingCanvasRef {
            ref_type: CanvasNodeRefKind::MediaAsset,
            ref_id,
            clear_node_ref: false,
        }
    })
}

fn config_id_in_lookup(config: &serde_json::Value, key: &str, ids: &HashSet<i64>) -> Option<i64> {
    config
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .filter(|id| ids.contains(id))
}
