mod bindings;
mod cleanup;
mod context;

pub use bindings::{
    bind_galgame_media_records, completed_galgame_visual_binding_records,
    unbind_galgame_media_records,
};
pub(in crate::galgame) use bindings::{
    galgame_media_binding_read_model, validate_galgame_media_asset_id,
    validate_galgame_media_target_ids,
};
pub(crate) use cleanup::delete_galgame_target_generation_media_graph_record;
pub use cleanup::delete_media_asset_with_provenance_records;
pub(in crate::galgame) use cleanup::{
    delete_galgame_line_generation_media_graph_record,
    delete_galgame_line_visual_generation_media_graph_record,
    delete_galgame_scene_visual_generation_media_graph_record,
};
#[cfg(test)]
pub(in crate::galgame) use context::validate_completed_visual_asset;
pub(in crate::galgame) use context::validate_completed_visual_asset_record;
#[cfg(test)]
pub(in crate::galgame) use context::validate_galgame_media_asset_source_context;
pub(in crate::galgame::media) use context::{
    galgame_bindable_media_asset, galgame_media_target_context_in,
};
pub use context::{
    galgame_generated_image_by_id, galgame_media_target_context,
    validate_galgame_portrait_position_samples,
};
pub(in crate::galgame) use context::{
    validate_galgame_portrait_spec_family_context, validate_galgame_portrait_spec_source_chapter,
    validate_galgame_portrait_spec_source_lines,
};
