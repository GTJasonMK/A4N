mod entities;
mod records;
mod required;
mod rules;

pub(crate) use entities::validate_entity_ref;
pub(crate) use records::{
    insert_generation_entity_ref, insert_generation_media_ref, insert_generation_text_anchor_ref,
};
pub(crate) use required::validate_required_prompt_refs;

#[cfg(test)]
pub(in crate::generation) use rules::{normalize_entity_kind, normalize_media_ref_role};
