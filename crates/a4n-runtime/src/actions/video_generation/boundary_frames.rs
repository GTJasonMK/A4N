mod lookup;
mod rules;

pub(super) use lookup::{BoundaryFrameImageIds, boundary_frame_image_ids_for_prompt};
pub(super) use rules::boundary_frame_numbers_for_prompt;

#[cfg(test)]
mod tests;
