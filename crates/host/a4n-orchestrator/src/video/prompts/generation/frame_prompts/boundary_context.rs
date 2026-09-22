mod assets;
mod generation;
mod style;

pub(in crate::video) use assets::{
    available_assets_with_previous_boundary_frame, backend_frame_reference_ids,
    frame_prompt_available_assets_context, previous_boundary_frame_asset_context,
    reference_slots_for_frame,
};
pub(in crate::video) use generation::boundary_frame_generation_context;
pub(super) use style::apply_boundary_frame_style_template;
#[cfg(test)]
pub(super) use style::frame_prompt_with_style_template;
