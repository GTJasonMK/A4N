mod prompts;
mod refs;

pub(in crate::project::duplicate) use prompts::copy_derived_prompt_artifacts;
pub(in crate::project::duplicate) use refs::copy_derived_generation_spec_media_refs;
