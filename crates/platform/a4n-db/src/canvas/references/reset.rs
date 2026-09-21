mod detection;
mod payload;
mod references;

#[cfg(test)]
pub(crate) use detection::reset_missing_canvas_ref_for_node;
pub(crate) use detection::{ResetTargetLookup, reset_missing_canvas_ref_for_fields_with_lookup};
#[cfg(test)]
pub(crate) use payload::clear_reset_generated_ref_payload;
pub(crate) use payload::clear_reset_generated_ref_payload_with_lookup;
#[cfg(test)]
pub(super) use references::canvas_node_references_generated_image;
pub(super) use references::canvas_node_references_generated_image_fields;
#[cfg(test)]
pub(super) use references::canvas_node_references_generated_video;
pub(crate) use references::generated_video_ref_project_ids;

#[cfg(test)]
mod tests;
