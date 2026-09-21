mod anchors;
mod prompts;
mod refs;
mod specs;

pub(super) use anchors::copy_derived_referenced_text_anchors;
pub(in crate::project::duplicate) use prompts::copy_derived_prompt_artifacts;
pub(in crate::project::duplicate) use refs::copy_derived_generation_spec_media_refs;
pub(super) use refs::{
    copy_derived_generation_spec_entity_refs, copy_derived_generation_spec_text_anchor_refs,
};
pub(super) use specs::copy_derived_generation_specs;
