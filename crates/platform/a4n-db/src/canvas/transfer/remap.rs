mod generation_targets;
mod ids;
mod metadata;
mod nodes;

#[cfg(test)]
pub(super) use generation_targets::remap_video_studio_generation_spec_target_entity_id;
pub(super) use generation_targets::{
    initial_video_studio_generation_spec_target_entity_id,
    remap_video_studio_generation_spec_target_entity_id_parts,
};
pub(super) use ids::{remap_video_studio_optional_id, remap_video_studio_required_id};
pub(super) use metadata::{
    remap_video_studio_canvas_cache_key, remap_video_studio_canvas_metadata,
    remap_video_studio_source_external_path,
};
pub(super) use nodes::remap_video_studio_canvas_node_refs;
