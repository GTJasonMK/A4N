use a4n_core::models::ai::image_gen_config;
use a4n_image::ImageGenerationRequest;
use serde_json::Value;

pub(super) const BATCH_IMAGE_ITEM_FIELDS: &[&str] = &[
    "prompt",
    "image_type",
    "chapter_id",
    "panel_id",
    "metadata_json",
    "config_id",
    "aspect_ratio",
    "reference_image_ids",
];

pub(in crate::actions::image_generation) struct BatchImageJob {
    pub(in crate::actions::image_generation) index: usize,
    pub(in crate::actions::image_generation) request: ImageGenerationRequest,
    pub(in crate::actions::image_generation) image_type: String,
    pub(in crate::actions::image_generation) preview_notes: Vec<String>,
    pub(in crate::actions::image_generation) chapter_id: Option<i64>,
    pub(in crate::actions::image_generation) panel_id: Option<String>,
    pub(in crate::actions::image_generation) metadata_json: Option<Value>,
    pub(in crate::actions::image_generation) config: image_gen_config::Model,
}

pub(super) struct OwnedVideoFrameImageGenerationContext {
    pub(super) source: String,
    pub(super) prompt_id: i64,
    pub(super) request_preview_note: Option<String>,
}

pub(in crate::actions::image_generation) fn batch_image_step_note(
    image_type: &str,
    preview_notes: &[String],
) -> String {
    let mut notes = Vec::with_capacity(preview_notes.len() + 1);
    notes.push(format!("image_type: {image_type}"));
    notes.extend(preview_notes.iter().cloned());
    notes.join("\n")
}
