mod cleanup;
mod config;
mod reset;

pub use cleanup::{
    mark_canvas_generated_image_refs_missing, mark_canvas_generated_video_refs_missing,
    mark_freeform_canvas_generated_refs_missing,
};
pub use reset::{
    canvas_node_references_generated_image, canvas_node_references_generated_video,
    clear_reset_generated_ref_payload, generated_video_ref_project_ids,
    reset_missing_canvas_ref_for_node,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingCanvasRef {
    pub ref_type: a4n_domain::CanvasNodeRefKind,
    pub ref_id: i64,
    pub clear_node_ref: bool,
}

#[cfg(test)]
use config::{IMAGE_REF_CONFIG_KEY, remove_config_any_i64_ref};

#[cfg(test)]
mod tests;
