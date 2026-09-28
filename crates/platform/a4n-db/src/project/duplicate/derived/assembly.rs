mod bindings;
pub(in crate::project::duplicate) use bindings::copy_derived_media_bindings;

mod clips;
pub(in crate::project::duplicate) use clips::copy_derived_assembly_clips;

mod rules;

#[cfg(test)]
mod tests;
