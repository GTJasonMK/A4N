mod bindings;
pub(in crate::project::duplicate) use bindings::copy_derived_media_bindings;

mod clips;
pub(in crate::project::duplicate) use clips::copy_derived_assembly_clips;

mod rules;

mod targets;
pub(super) use targets::update_derived_generation_spec_segment_targets;

mod timelines;
pub(super) use timelines::copy_derived_assembly_timelines;

#[cfg(test)]
mod tests;
