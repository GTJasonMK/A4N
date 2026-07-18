mod fixups;
mod generation_specs;
mod graphs;
mod text_assets;

pub(super) use fixups::{
    update_freeform_generation_spec_deferred_targets_record,
    update_freeform_generation_spec_metadata_record,
};
pub(super) use generation_specs::copy_freeform_generation_specs_record;
pub(super) use graphs::copy_freeform_canvas_graphs_record;
pub(super) use text_assets::copy_freeform_canvas_text_assets_record;
